#![no_std]
#![no_main]
#![doc = include_str!("../README.md")]

use panic_rtt_target as _;
use rtic::app;

mod buttons;
mod can;
mod render;
mod sd;
mod spi;
mod state;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [TIM2, TIM3, TIM4])]
mod app {
    use bxcan::Frame;
    use embedded_sdmmc as sdmmc;
    use fugit::Instant;
    use heapless::{
        spsc::{Consumer, Producer, Queue},
        String, Vec,
    };
    use rtic_monotonics::systick::prelude::*;
    use rtt_target::{rprintln, rtt_init_print};
    use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
    use stm32f1xx_hal::{
        can::Can,
        flash::FlashExt,
        gpio::{ExtiPin, Output, Pin},
        i2c::{self, BlockingI2c},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, Phase, Polarity, Spi},
    };

    use crate::{
        buttons::*,
        can::*,
        render::{
            draw_header, flush_text_line, formatted_string, TEXT_LINE_2, TEXT_LINE_3, TEXT_LINE_4,
            TEXT_LINE_5,
        },
        sd::*,
        spi::*,
        state::*,
    };

    /// Frequency of external oscillator.
    ///
    /// See [`stm32f1xx_hal::rcc::CFGR::use_hse()`] for details.
    pub const HSE_CLOCK_RATE_MHZ: u32 = 8;
    /// MCU main system Clock Rate.
    ///
    /// Can be between 8 and 72 MHz. Check the STM32F103Cx documentation for details.
    ///
    /// See [`stm32f1xx_hal::rcc::CFGR::sysclk()`] for details.
    pub const SYS_CLOCK_RATE_MHZ: u32 = 64;
    const _: () = assert!(8 <= SYS_CLOCK_RATE_MHZ && SYS_CLOCK_RATE_MHZ <= 72);
    /// Frequency of the PCLK1 clock.
    ///
    /// Defines the upper bound for SPI clock rates.
    ///
    /// See [`stm32f1xx_hal::rcc::CFGR::pclk1()`] for details.
    pub const PCLK1_CLOCK_RATE_MHZ: u32 = 16;
    /// The tick rate of the timer peripheral in Hz.
    ///
    /// This defines the precision of the time managing [Monotonic](Mono).
    pub const TICK_RATE: u32 = 1_000;

    /// Capacity of the CAN TX queue.
    ///
    /// This queue serves as a buffer for SD reading operations. It gets filled by [`sd_reader()`]
    /// and is consumed by [`can_sender()`].
    pub const CAN_TX_QUEUE_CAPACITY: usize = 8;
    /// Capacity of the CAN RX queue.
    ///
    /// This queue serves as a buffer for SD writing operations. It gets filled by [`can_receiver()`]
    /// and is consumed by [`sd_writer()`]. It's important to make the queue large as SD writing operations
    /// are much slower than a full speed CAN bus.
    pub const SD_RX_QUEUE_CAPACITY: usize = 64;

    /// Debouncing delay applied to button inputs.
    ///
    /// Button presses for a same button closer that [`DEBOUNCE_DELAY_MS`] will be ignored.
    pub const DEBOUNCE_DELAY_MS: u32 = 100;

    /// SPI clock rate for the SD interface.
    ///
    /// Must be lower than [`PCLK1_CLOCK_RATE_MHZ`].
    pub const SD_SPI_CLK_MHZ: u32 = 16;
    const _: () = assert!(SD_SPI_CLK_MHZ <= PCLK1_CLOCK_RATE_MHZ);
    /// Maximum number of file from a single directory than can be loaded at once.
    ///
    /// If a directory contains more than [`MAX_SD_INDEX_AMOUNT`] files, only the first [`MAX_SD_INDEX_AMOUNT`]
    /// files will be displayed.
    pub const MAX_SD_INDEX_AMOUNT: usize = 32;
    /// Maximum indexing depth of the SD indexer.
    ///
    /// Opening a directory in the file explorer increases the depth of the indexer. File or directories
    /// selected beyond the [`MAX_SD_INDEX_DEPTH`]th directory will not be able to be opened.
    pub const MAX_SD_INDEX_DEPTH: usize = 8;

    systick_monotonic!(Mono, TICK_RATE);

    #[shared]
    struct Shared {
        /// Wrapped CAN bus manager.
        can: CanContext,
        /// Wrapped buttons manager.
        #[lock_free]
        button_panel: ButtonPanel,
        /// SD card volume manager.
        volume_manager: VolumeManager,
        /// System state manager, wraps a [`Display`](crate::render::Display) and [`State`](State).
        state_manager: StateManager,
    }

    #[local]
    struct Local {
        /// Producer end of the CAN TX queue. Used by [`sd_reader()`].
        can_tx_producer: Producer<'static, Frame, CAN_TX_QUEUE_CAPACITY>,
        /// Consumer end of the CAN TX queue. Used by [`can_sender()`].
        can_tx_consumer: Consumer<'static, Frame, CAN_TX_QUEUE_CAPACITY>,
        /// Producer end of the CAN RX queue. Used by [`can_receiver()`].
        can_rx_producer: Producer<'static, Frame, SD_RX_QUEUE_CAPACITY>,
        /// Consumer end of the CAN RX queue. Used by [`sd_writer()`].
        can_rx_consumer: Consumer<'static, Frame, SD_RX_QUEUE_CAPACITY>,
        /// Status LED control pin. Used by [`blinker()`].
        status_led: Pin<'C', 15, Output>,
    }

    /// Initialisation function.
    ///
    /// This routine :
    /// - Configures system clocks
    /// - Configures GPIOs (buttons and LED)
    /// - Initialises the Display
    /// - Sets up the CAN transceiver
    /// - Initialises the SD card
    /// - Sets up the State Manager
    ///
    /// Improvements could be made to this routine, like :
    /// - Display boot error message, especially for the SD care initialisation that is quite error prone
    /// - Make SD initialisation non blocking so that the home screen can be displayed early
    #[init(
        local = [
            q_tx: Queue<Frame, CAN_TX_QUEUE_CAPACITY> = Queue::new(),
            q_rx: Queue<Frame, SD_RX_QUEUE_CAPACITY> = Queue::new(),
        ]
    )]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("Initializing...");

        // Init flash, RCC and clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(HSE_CLOCK_RATE_MHZ.MHz())
            .sysclk(SYS_CLOCK_RATE_MHZ.MHz())
            .hclk(SYS_CLOCK_RATE_MHZ.MHz())
            .pclk1(PCLK1_CLOCK_RATE_MHZ.MHz())
            .pclk2(SYS_CLOCK_RATE_MHZ.MHz())
            .freeze(&mut flash.acr);
        Mono::start(cx.core.SYST, SYS_CLOCK_RATE_MHZ * 1_000_000);

        let mut gpioa = cx.device.GPIOA.split();
        let mut gpiob = cx.device.GPIOB.split();
        let mut gpioc = cx.device.GPIOC.split();
        let mut afio = cx.device.AFIO.constrain();

        // Init Display
        rprintln!("-> Display");
        let display_i2c = BlockingI2c::i2c1(
            cx.device.I2C1,
            (
                gpiob.pb6.into_alternate_open_drain(&mut gpiob.crl), // scl
                gpiob.pb7.into_alternate_open_drain(&mut gpiob.crl), // sda
            ),
            &mut afio.mapr,
            i2c::Mode::Fast {
                frequency: 400_000.Hz(),
                duty_cycle: i2c::DutyCycle::Ratio2to1,
            },
            clocks,
            1000,
            10,
            1000,
            1000,
        );
        let interface = I2CDisplayInterface::new(display_i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        while let Err(_) = display.init() {}

        draw_header(&mut display, "Booting...", true);
        let _ = display.flush();

        // Init CAN bus
        rprintln!("-> CAN bus");
        flush_text_line(&mut display, "-> CAN bus", TEXT_LINE_2);
        let can = CanContext::new(
            Can::new(cx.device.CAN1, cx.device.USB),
            gpiob.pb8.into_floating_input(&mut gpiob.crh), // can rx
            gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh), // can tx
            &mut afio.mapr,
        );

        // Init CAN TX & RX queues
        let (can_tx_producer, can_tx_consumer) = cx.local.q_tx.split();
        let (can_rx_producer, can_rx_consumer) = cx.local.q_rx.split();

        // Init buttons
        rprintln!("-> Buttons");
        flush_text_line(&mut display, "-> Buttons", TEXT_LINE_3);
        let (_pa15, _pb3, pb4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);
        let mut button_panel = ButtonPanel {
            button_ok: pb4.into_pull_up_input(&mut gpiob.crl),
            button_up: gpioa.pa0.into_pull_up_input(&mut gpioa.crl),
            button_down: gpioa.pa1.into_pull_up_input(&mut gpioa.crl),
            button_right: gpioa.pa2.into_pull_up_input(&mut gpioa.crl),
            button_left: gpioa.pa3.into_pull_up_input(&mut gpioa.crl),
        };
        button_panel.enable_interrupts(&mut afio, &mut cx.device.EXTI);

        // Init status LED
        rprintln!("-> LED");
        flush_text_line(&mut display, "-> Status LED & blink", TEXT_LINE_4);
        let status_led = gpioc.pc15.into_push_pull_output(&mut gpioc.crh);
        blinker::spawn().expect("Blinker wasn't started yet.");

        // Init SD Card
        rprintln!("-> SD Card");
        flush_text_line(&mut display, "-> SD card", TEXT_LINE_5);
        let volume_manager = {
            let sd_spi = SpiWrapper {
                spi: Spi::spi2(
                    cx.device.SPI2,
                    (
                        gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh), // sck
                        gpiob.pb14,                                          // miso
                        gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh), // mosi
                    ),
                    Mode {
                        phase: Phase::CaptureOnSecondTransition,
                        polarity: Polarity::IdleHigh,
                    },
                    SD_SPI_CLK_MHZ.MHz(),
                    clocks,
                ),
            };
            let sd_card = sdmmc::SdCard::new(
                sd_spi,
                OutputPinWrapper {
                    pin: gpiob.pb12.into_push_pull_output(&mut gpiob.crh), // cs
                },
                Mono,
            );
            rprintln!("Found SD card with size {:?}", sd_card.num_bytes());

            sdmmc::VolumeManager::<_, _, 2, 2, 1>::new_with_limits(sd_card, FakeTimeSource {}, 5000)
        };

        // Init StateManager
        let mut state_manager = StateManager::default_with_display(display);
        state_manager.render();
        state_updater::spawn().expect("State updater wasn't started yet.");

        rprintln!("Initialisation done");

        (
            Shared {
                can,
                button_panel,
                volume_manager,
                state_manager,
            },
            Local {
                can_tx_producer,
                can_tx_consumer,
                can_rx_producer,
                can_rx_consumer,
                status_led,
            },
        )
    }

    /// Function responsible of the Status LED.
    ///
    /// Blinks slowly while idle and rapidly while running.
    ///
    /// Should always have priority to ensure blinking during otherwise blocking operations.
    #[task(
        priority = 12,
        shared = [state_manager],
        local = [status_led],
    )]
    async fn blinker(mut cx: blinker::Context) {
        loop {
            let delay = cx.shared.state_manager.lock(|sm| {
                if sm.state.running {
                    100.millis()
                } else {
                    500.millis()
                }
            });
            Mono::delay(delay).await;

            cx.local.status_led.toggle();
        }
    }

    /// Function sending queued CAN frames.
    ///
    /// It triggers with the [`USB_HP_CAN_TX()`] interrupt and empties the CAN TX Queue. The
    /// interrupt must be enabled for this function to trigger.
    ///
    /// The [`USB_HP_CAN_TX()`] interrupt must be triggered the first time items are added to the Queue
    /// to initiate transmission. Every successful transmission will trigger the interrupt again and
    /// thus consume the CAN TX Queue until empty.
    #[task(
        binds = USB_HP_CAN_TX,
        priority = 4,
        shared = [can, state_manager],
        local = [can_tx_consumer]
    )]
    fn can_sender(mut cx: can_sender::Context) {
        let mut can = cx.shared.can;
        let tx_queue = cx.local.can_tx_consumer;

        can.lock(|can| {
            can.bus.clear_tx_interrupt();

            if can.bus.is_transmitter_idle() {
                while let Some(frame) = tx_queue.peek() {
                    rprintln!("Transmitting {:?}", frame);
                    match can.bus.transmit(&frame) {
                        Ok(status) => {
                            tx_queue.dequeue();
                            assert_eq!(
                                status.dequeued_frame(),
                                None,
                                "All mailboxes should have been empty"
                            );
                            cx.shared
                                .state_manager
                                .lock(|sm| sm.state.success_count += 1);
                        }
                        Err(nb::Error::WouldBlock) => break,
                        Err(_) => unreachable!(),
                    }
                }
            }
        });
    }

    /// Function queuing received CAN frames for SD writing.
    ///
    /// It triggers with the [`USB_LP_CAN_RX0()`] interrupt and fills the SD RX Queue. The interrupt
    /// must be enabled for this function to trigger.
    ///
    /// If the SD RX Queue is full, the received frame will be dumped and a warning will be emitted.
    #[task(
        binds = USB_LP_CAN_RX0,
        priority = 5,
        shared = [can],
        local = [can_rx_producer],
    )]
    fn can_receiver(mut cx: can_receiver::Context) {
        let rx_queue = cx.local.can_rx_producer;
        cx.shared.can.lock(|can| {
            if let Ok(frame) = can.bus.receive() {
                rprintln!("Received {:?}", frame);
                if rx_queue.ready() {
                    rx_queue.enqueue(frame).expect("rx_queue is ready.");
                } else {
                    rprintln!("WARNING - Couldn't queue a frame for writing");
                }
            }
        });
    }

    /// Function handling OK button inputs.
    ///
    /// It is triggered by the [`EXTI4()`] interrupt which can be triggered by any enabled Px4 pin (PA4,
    /// PB4, PC4). See [`init()`] for details on enabled pins.
    ///
    /// It first checks for debouncing then updates the [`StateManager`].
    #[task(
        binds = EXTI4,
        priority = 8,
        shared = [button_panel, state_manager],
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
    )]
    fn clicked_ok(mut cx: clicked_ok::Context) {
        cx.shared
            .button_panel
            .button_ok
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed OK");
        cx.shared.state_manager.lock(|sm| sm.press(Button::Ok));
        let _ = state_updater::spawn();
    }

    /// Function handling UP button inputs.
    ///
    /// It is triggered by the [`EXTI0()`] interrupt which can be triggered by any enabled Px0 pin (PA0,
    /// PB0, PC0, PD0). See [`init()`] for details on enabled pins.
    ///
    /// It first checks for debouncing then updates the [`StateManager`].
    #[task(
        binds = EXTI0,
        priority = 8,
        shared = [button_panel, state_manager],
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
    )]
    fn clicked_up(mut cx: clicked_up::Context) {
        cx.shared
            .button_panel
            .button_up
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed UP");
        cx.shared.state_manager.lock(|sm| sm.press(Button::Up));
        let _ = state_updater::spawn();
    }

    /// Function handling DOWN button inputs.
    ///
    /// It is triggered by the [`EXTI1()`] interrupt which can be triggered by any enabled Px1 pin (PA1,
    /// PB1, PC1, PD1). See [`init()`] for details on enabled pins.
    ///
    /// It first checks for debouncing then updates the [`StateManager`].
    #[task(
        binds = EXTI1,
        priority = 8,
        shared = [button_panel, state_manager],
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
    )]
    fn clicked_down(mut cx: clicked_down::Context) {
        cx.shared
            .button_panel
            .button_down
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed DOWN");
        cx.shared.state_manager.lock(|sm| sm.press(Button::Down));
        let _ = state_updater::spawn();
    }

    /// Function handling RIGHT button inputs.
    ///
    /// It is triggered by the [`EXTI2()`] interrupt which can be triggered by any enabled Px2 pin (PA2,
    /// PB2, PC2). See [`init()`] for details on enabled pins.
    ///
    /// It first checks for debouncing then updates the [`StateManager`].
    #[task(
        binds = EXTI2,
        priority = 8,
        shared = [button_panel, state_manager],
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
    )]
    fn clicked_right(mut cx: clicked_right::Context) {
        cx.shared
            .button_panel
            .button_right
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed RIGHT");
        cx.shared.state_manager.lock(|sm| sm.press(Button::Right));
        let _ = state_updater::spawn();
    }

    /// Function handling LEFT button inputs.
    ///
    /// It is triggered by the [`EXTI3()`] interrupt which can be triggered by any enabled Px3 pin (PA3,
    /// PB3, PC3). See [`init()`] for details on enabled pins.
    ///
    /// It first checks for debouncing then updates the [`StateManager`].
    #[task(
        binds = EXTI3,
        priority = 8,
        shared = [button_panel, state_manager],
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
    )]
    fn clicked_left(mut cx: clicked_left::Context) {
        cx.shared
            .button_panel
            .button_left
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed LEFT");
        cx.shared.state_manager.lock(|sm| sm.press(Button::Left));
        let _ = state_updater::spawn();
    }

    /// Function propagating updates done to the [`StateManager`].
    ///
    /// It matches the current screen being displayed and the state of the system to determine which
    /// function to spawn of interrupt to set up. It handles starting and stopping reading and writing
    /// operations and thus has higher priority than all other blocking operations. If necessary, it
    /// will also trigger a [render](StateManager::render()).
    ///
    /// It can be called after user inputs (buttons) or when a reading or writing operation finishes.
    #[task(
        priority = 7,
        shared = [state_manager, can],
    )]
    async fn state_updater(cx: state_updater::Context) {
        (cx.shared.state_manager, cx.shared.can).lock(|sm, can| {
            match (sm.current_screen(), &sm.state) {
                (Screen::EmissionSelection { .. }, State { running: true, .. }) => {
                    let _ = sd_indexer::spawn(false); // If sd_indexer is already running, just wait for it to finish
                }
                (Screen::CaptureSelection { .. }, State { running: true, .. }) => {
                    let _ = sd_indexer::spawn(true); // If sd_indexer is already running, just wait for it to finish
                }
                (
                    Screen::EmissionSelection { .. } | Screen::CaptureSelection { .. },
                    State { running: false, .. },
                ) => {
                    sm.render();
                }
                (
                    Screen::Emission,
                    State {
                        running: true,
                        bitrate,
                        emission_mode,
                        ..
                    },
                ) => {
                    can.enable_tx(*bitrate, *emission_mode);
                    sd_reader::spawn()
                        .expect("sd_reader shouldn't be running (running was false).");
                }
                (
                    Screen::Capture,
                    State {
                        running: true,
                        bitrate,
                        capture_silent,
                        ..
                    },
                ) => {
                    can.enable_rx(*bitrate, *capture_silent);
                    let _ = sd_writer::spawn(); // Can be already spawned since [`state_updater()`] will be called again if a button other than OK is pressed.
                }
                (Screen::Emission | Screen::Capture, State { running: false, .. }) => {
                    can.disable();
                    sm.render();
                }
                _ => {}
            }
        });
    }

    /// Function indexing the Micro SD.
    ///
    /// When called, it will read the path to index from [`State::dir_path`] and populate
    /// [`State::dir_content`] with the index. It will only index the first [`MAX_SD_INDEX_AMOUNT`]
    /// files and folder found.
    ///
    /// The files and folder indexed are sorted with directories first and then by alphabetical
    /// order. See [`index_dir()`] for implementation details.
    #[task(
        priority = 1,
        shared = [volume_manager, state_manager],
        // local = [],
    )]
    async fn sd_indexer(cx: sd_indexer::Context, dirs_only: bool) {
        (cx.shared.volume_manager, cx.shared.state_manager).lock(|vm, sm| {
            let mut sd_volume = vm.open_volume(sdmmc::VolumeIdx(0)).unwrap();
            let mut dir = sd_volume.open_root_dir().unwrap();

            for dir_name in &sm.state.dir_path {
                dir.change_dir(dir_name)
                    .expect("Path only contains existing items.");
            }

            sm.state.dir_content = Vec::new();
            index_dir(&mut dir, &mut sm.state.dir_content, dirs_only).unwrap();

            rprintln!("{:?}", sm.state.dir_content);
            sm.state.running = false;
        });

        state_updater::spawn()
            .expect("state_updater should not be running (it has higher priority)");
    }

    /// Function reading CAN frames from a file on the Micro SD.
    ///
    /// When called, it will resolve the path given in [`State::dir_path`] and start reading the
    /// file's content. See [`sd::CanLogsIterator`] for implementation details. It will loop over the
    /// file [`State::emission_count`] times except if it is `0`, in which case it will loop until
    /// [`State::running`] is set to `false`. The frames read from the file will be queued to the CAN
    /// TX Queue to be read by [can_sender()].
    ///
    /// Once reading is done, [`State::running`] will be set to false and [`state_updater()`] will be
    /// called.
    #[task(
        priority = 1,
        shared = [volume_manager, state_manager],
        local = [can_tx_producer],
    )]
    async fn sd_reader(mut cx: sd_reader::Context) {
        let tx_queue = cx.local.can_tx_producer;
        let mut emission_count = match cx.shared.state_manager.lock(|sm| sm.state.emission_count) {
            0 => None,
            n => Some(n),
        };

        cx.shared.volume_manager.lock(|vm| {
            let mut sd_volume = vm.open_volume(sdmmc::VolumeIdx(0)).unwrap();

            let (file, mut dir) = cx.shared.state_manager.lock(|sm| {
                let mut dir = sd_volume.open_root_dir().unwrap();
                let (file, path) = sm
                    .state
                    .dir_path
                    .split_last()
                    .expect("Path should have been filled before calling sd_reader.");
                for dir_name in path {
                    dir.change_dir(dir_name)
                        .expect("Path only contains existing items.");
                }

                (file.clone(), dir)
            });

            let mut get_running = || cx.shared.state_manager.lock(|sm| sm.state.running); // Function alias for code readability

            while emission_count.unwrap_or(u8::MAX) > 0 && get_running() {
                let logs = CanLogsIterator::new(
                    dir.open_file_in_dir(&file, sdmmc::Mode::ReadOnly)
                        .expect("Path only contains existing items."),
                );

                for frame in logs {
                    while !tx_queue.ready() && get_running() {}
                    if !get_running() {
                        break;
                    }
                    enqueue_frame(tx_queue, frame).expect("tx_queue is ready.");
                }

                if let Some(ref mut n) = emission_count {
                    *n -= 1;
                }
            }

            while tx_queue.len() != 0 && get_running() {} // Wait here for queue to be empty to prevent early `running = false`
        });

        cx.shared.state_manager.lock(|sm| sm.state.running = false);
        state_updater::spawn()
            .expect("state_updater should not be running (it has higher priority)");
    }

    /// Function writing received CAN frames to the Micro SD.
    ///
    /// When called, it will resolve the path given in [`State::dir_path`] and create a file in the
    /// found folder. It will then wait for frames to be queued in the SD RX Queue. Queued frames
    /// will then be poped and written on the Micro SD.
    ///
    /// When [`State::running`] is set to false, the Queue will be emptied and written in the file
    /// before exiting. This is to prevent too many frames from being lost due to slowness of SD
    /// writing compared to CAN reading, making [`sd_writer()`] late compared to [`can_receiver()`].
    #[task(
        priority = 1,
        shared = [volume_manager, state_manager],
        local = [can_rx_consumer],
    )]
    async fn sd_writer(mut cx: sd_writer::Context) {
        let rx_queue = cx.local.can_rx_consumer;

        cx.shared.volume_manager.lock(|vm| {
            let mut sd_volume = vm.open_volume(sdmmc::VolumeIdx(0)).unwrap();

            let mut dir = cx.shared.state_manager.lock(|sm| {
                let mut dir = sd_volume.open_root_dir().unwrap();
                for dir_name in &sm.state.dir_path {
                    dir.change_dir(dir_name)
                        .expect("Path only contains existing items.");
                }

                dir
            });

            let file_name: String<12> =
                formatted_string(format_args!("{:08}.log", Mono::now().ticks()))
                    .expect("Formatted args should fit.");
            let mut logs = dir
                .open_file_in_dir(&file_name[..], sdmmc::Mode::ReadWriteCreateOrTruncate)
                .unwrap();

            rprintln!("Writing started to '{}'", file_name);
            let (bitrate, silent) = cx
                .shared
                .state_manager
                .lock(|sm| (sm.state.bitrate, sm.state.capture_silent));
            let _ = logs.write(
                formatted_string::<64>(format_args!(
                    "# Frames captured by CANary - Bitrate: {:4} kbps, Silent: {}\n",
                    bitrate as u32 / 1000,
                    silent
                ))
                .expect("Formatted args should fit.")
                .as_bytes(),
            );

            while cx.shared.state_manager.lock(|sm| sm.state.running) || rx_queue.ready() {
                if let Some(frame) = rx_queue.dequeue() {
                    rprintln!("Writing {:?}", frame);
                    if let Err(_) = logs.write(frame_to_log(&frame).as_bytes()) {
                        rprintln!("Got error on writing ");
                    } else {
                        cx.shared
                            .state_manager
                            .lock(|sm| sm.state.success_count += 1);
                    };
                }
            }
        });

        rprintln!("Writing stopped");
    }
}
