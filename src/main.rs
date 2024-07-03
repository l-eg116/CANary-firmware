#![no_std]
#![no_main]
#![allow(dead_code)]

use panic_rtt_target as _;
use rtic::app;

mod buttons;
mod can;
mod display;
mod sd;
mod spi;
mod status;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [TIM2, TIM3, TIM4])]
mod app {
    use core::fmt::Write;

    use bxcan::Frame;
    use embedded_sdmmc as sdmmc;
    use fugit::Instant;
    use heapless::{
        spsc::{Consumer, Producer, Queue},
        String,
    };
    use rtic_monotonics::systick::prelude::*;
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f1xx_hal::{
        can::Can,
        flash::FlashExt,
        gpio::{ExtiPin, Output, Pin},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, Phase, Polarity, Spi},
    };

    use crate::{buttons::*, can::*, display::*, sd::*, spi::*, status::*};

    pub const CAN_TX_QUEUE_CAPACITY: usize = 8;
    pub const SD_RX_QUEUE_CAPACITY: usize = 64;
    pub const CLOCK_RATE_MHZ: u32 = 64;
    pub const TICK_RATE: u32 = 1_000;
    pub const DEBOUNCE_DELAY_MS: u32 = 10;

    systick_monotonic!(Mono, TICK_RATE);

    #[shared]
    struct Shared {
        can: CanContext,
        #[lock_free]
        button_panel: ButtonPanel,
        status: CanaryStatus,
        volume_manager: VolumeManager,
        display_manager: DisplayManager,
    }

    #[local]
    struct Local {
        can_tx_producer: Producer<'static, Frame, CAN_TX_QUEUE_CAPACITY>,
        can_tx_consumer: Consumer<'static, Frame, CAN_TX_QUEUE_CAPACITY>,
        can_rx_producer: Producer<'static, Frame, SD_RX_QUEUE_CAPACITY>,
        can_rx_consumer: Consumer<'static, Frame, SD_RX_QUEUE_CAPACITY>,
        status_led: Pin<'C', 15, Output>,
    }

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
            .use_hse(8.MHz())
            .sysclk(CLOCK_RATE_MHZ.MHz())
            .hclk(CLOCK_RATE_MHZ.MHz())
            .pclk1(16.MHz())
            .pclk2(CLOCK_RATE_MHZ.MHz())
            .freeze(&mut flash.acr);
        Mono::start(cx.core.SYST, CLOCK_RATE_MHZ * 1_000_000);

        let mut gpioa = cx.device.GPIOA.split();
        let mut gpiob = cx.device.GPIOB.split();
        let mut gpioc = cx.device.GPIOC.split();
        let mut afio = cx.device.AFIO.constrain();

        // Init CAN bus
        rprintln!("-> CAN bus");
        let mut can = CanContext::new(
            Can::new(cx.device.CAN1, cx.device.USB),
            gpiob.pb8.into_floating_input(&mut gpiob.crh), // can rx
            gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh), // can tx
            &mut afio.mapr,
        );
        can.set_bitrate(Bitrate::Br125kbps);
        can.enable_interrupts();
        can.enable_non_blocking();

        // Init CAN TX & RX queues
        let (can_tx_producer, can_tx_consumer) = cx.local.q_tx.split();
        let (can_rx_producer, can_rx_consumer) = cx.local.q_rx.split();

        // Init buttons
        rprintln!("-> Buttons");
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
        let status_led = gpioc.pc15.into_push_pull_output(&mut gpioc.crh);
        let status = CanaryStatus::Idle;
        blinker::spawn().unwrap();

        // Init SD Card
        rprintln!("-> SD Card");
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
                    2.MHz(),
                    clocks,
                ),
            };
            let sd_card = embedded_sdmmc::SdCard::new(
                sd_spi,
                OutputPinWrapper {
                    pin: gpiob.pb12.into_push_pull_output(&mut gpiob.crh), // cs
                },
                Mono,
            );
            rprintln!("Found SD card with size {:?}", sd_card.num_bytes());

            sdmmc::VolumeManager::<_, _, 2, 2, 1>::new_with_limits(sd_card, FakeTimeSource {}, 5000)
        };

        // Init DisplayManager
        let display_manager = DisplayManager::default();

        (
            Shared {
                can,
                button_panel,
                status,
                volume_manager,
                display_manager,
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

    #[idle]
    fn idle(_: idle::Context) -> ! {
        rprintln!("Entering idle loop");
        loop {
            // asm::wfi();
        }
    }

    #[task(
        priority = 1,
        shared = [status],
        local = [status_led],
    )]
    async fn blinker(mut cx: blinker::Context) {
        loop {
            Mono::delay(cx.shared.status.lock(|status| match status {
                CanaryStatus::Idle => 500.millis(),
                CanaryStatus::Active => 100.millis(),
                CanaryStatus::InfoBlink(0) => {
                    *status = CanaryStatus::Idle;
                    10.millis()
                }
                CanaryStatus::InfoBlink(n) => {
                    *status = CanaryStatus::InfoBlink(*n - 1);
                    10.millis()
                }
            }))
            .await;

            cx.local.status_led.toggle();
        }
    }

    #[task(
        binds = USB_HP_CAN_TX,
        priority = 4,
        shared = [can],
        local = [can_tx_consumer]
    )]
    fn can_sender(cx: can_sender::Context) {
        let mut can = cx.shared.can;
        let tx_queue = cx.local.can_tx_consumer;

        can.lock(|can| {
            can.bus.clear_tx_interrupt();

            if can.bus.is_transmitter_idle() {
                while let Some(frame) = tx_queue.peek() {
                    rprintln!("Transmiting {:?}", frame);
                    match can.bus.transmit(&frame) {
                        Ok(status) => {
                            tx_queue.dequeue();
                            assert_eq!(
                                status.dequeued_frame(),
                                None,
                                "All mailboxes should have been empty"
                            );
                        }
                        Err(nb::Error::WouldBlock) => break,
                        Err(_) => unreachable!(),
                    }
                }
            }
        });
    }

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
                    rx_queue.enqueue(frame).expect("rx_queue is ready");
                } else {
                    rprintln!("WARNING - Couldn't queue a frame for writing");
                }
            }
        });
    }

    #[task(
        binds = EXTI4,
        priority = 8,
        shared = [button_panel, display_manager],
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
        cx.shared.display_manager.lock(|dm| dm.press(Button::Ok));
    }

    #[task(
        binds = EXTI0,
        priority = 8,
        shared = [button_panel, display_manager],
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
        cx.shared.display_manager.lock(|dm| dm.press(Button::Up));
    }

    #[task(
        binds = EXTI1,
        priority = 8,
        shared = [button_panel, display_manager],
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
        cx.shared.display_manager.lock(|dm| dm.press(Button::Down));
    }

    #[task(
        binds = EXTI2,
        priority = 8,
        shared = [button_panel, display_manager],
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
        cx.shared.display_manager.lock(|dm| dm.press(Button::Right));
    }

    #[task(
        binds = EXTI3,
        priority = 8,
        shared = [button_panel, display_manager],
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
        cx.shared.display_manager.lock(|dm| dm.press(Button::Left));
    }

    #[task(
        priority = 2,
        shared = [volume_manager, display_manager],
        local = [can_tx_producer],
    )]
    async fn sd_reader(mut cx: sd_reader::Context, file_name: &str) {
        let tx_queue = cx.local.can_tx_producer;
        let mut is_running = || cx.shared.display_manager.lock(|dm| dm.state.running);

        cx.shared.volume_manager.lock(|vm| {
            let mut sd_volume = vm.open_volume(sdmmc::VolumeIdx(0)).unwrap();
            let mut root_dir = sd_volume.open_root_dir().unwrap();
            let logs = CanLogsInterator::new(
                root_dir
                    .open_file_in_dir(file_name, sdmmc::Mode::ReadOnly)
                    .unwrap(),
            );

            for frame in logs {
                while !tx_queue.ready() && is_running() {}
                if !is_running() {
                    break;
                }
                enqueue_frame(tx_queue, frame).expect("tx_queue is ready");
            }
        });
    }

    #[task(
        priority = 1,
        shared = [volume_manager, display_manager],
        local = [can_rx_consumer],
    )]
    async fn sd_writer(mut cx: sd_writer::Context) {
        let rx_queue = cx.local.can_rx_consumer;
        let mut is_running = || cx.shared.display_manager.lock(|dm| dm.state.running);

        // while let Some(_) = rx_queue.dequeue() {} // Queue already empty

        cx.shared.volume_manager.lock(|vm| {
            let mut sd_volume = vm.open_volume(sdmmc::VolumeIdx(0)).unwrap();
            let mut root_dir = sd_volume.open_root_dir().unwrap();

            let mut file_name = String::<12>::new();
            file_name
                .write_fmt(format_args!("{:08}.log", Mono::now().ticks()))
                .unwrap();
            let mut logs = root_dir
                .open_file_in_dir(&file_name[..], sdmmc::Mode::ReadWriteCreateOrTruncate)
                .unwrap();

            rprintln!("Writing started to '{}'", file_name);

            while is_running() {
                if let Some(frame) = rx_queue.dequeue() {
                    rprintln!("Writing {:?}", frame);
                    if let Err(_) = logs.write(frame_to_log(&frame).as_bytes()) {
                        rprintln!("Got error on writing ");
                    };
                }
            }
        });
        while let Some(_) = rx_queue.dequeue() {} // Empty Queue for next time
        rprintln!("Writing stopped");
    }
}
