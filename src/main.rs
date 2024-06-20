#![no_std]
#![no_main]
#![allow(dead_code)]

use panic_rtt_target as _;
use rtic::app;

mod buttons;
mod can;
mod spi;
mod status;

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [TIM2])]
mod app {
    use bxcan::Frame;
    use cortex_m::asm;
    use embedded_sdmmc as sdmmc;
    use fugit::Instant;
    use heapless::spsc::{Consumer, Producer, Queue};
    use rtic_monotonics::systick::prelude::*;
    use rtt_target::{rprintln, rtt_init_print};
    use stm32f1xx_hal::{
        can::Can,
        flash::FlashExt,
        gpio::{Alternate, ExtiPin, Output, Pin},
        prelude::*,
        rcc::RccExt,
        spi::{Mode, Phase, Polarity, Spi},
    };

    use crate::{buttons::*, can::*, spi::*, status::*};

    pub const CAN_TX_CAPACITY: usize = 8;
    pub const CLOCK_RATE_MHZ: u32 = 8;
    pub const TICK_RATE: u32 = 1_000;
    pub const DEBOUNCE_DELAY_MS: u32 = 10;

    systick_monotonic!(Mono, TICK_RATE);

    #[shared]
    struct Shared {
        can: CanContext,
        #[lock_free]
        controller: Controller,
        can_tx_producer: Producer<'static, Frame, CAN_TX_CAPACITY>,
        status: CanaryStatus,
        #[lock_free]
        volume_manager: sdmmc::VolumeManager<
            sdmmc::SdCard<
                SpiWrapper<(
                    Pin<'B', 13, Alternate>,
                    Pin<'B', 14>,
                    Pin<'B', 15, Alternate>,
                )>,
                OutputPinWrapper<'B', 12>,
                Mono,
            >,
            FakeTimeSource,
            2,
            2,
        >,
    }

    #[local]
    struct Local {
        can_tx_consumer: Consumer<'static, Frame, CAN_TX_CAPACITY>,
        status_led: Pin<'C', 13, Output>,
    }

    #[init(local = [q: Queue<Frame, CAN_TX_CAPACITY> = Queue::new(),])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        rtt_init_print!();
        rprintln!("Initializing...");

        // Init flash, RCC and clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(CLOCK_RATE_MHZ.MHz())
            .freeze(&mut flash.acr);
        Mono::start(cx.core.SYST, CLOCK_RATE_MHZ * 1_000_000);

        let mut gpioa = cx.device.GPIOA.split();
        let mut gpiob = cx.device.GPIOB.split();
        let mut gpioc = cx.device.GPIOC.split();
        let mut afio = cx.device.AFIO.constrain();

        // Init CAN bus
        let mut can = CanContext::new(
            Can::new(cx.device.CAN1, cx.device.USB),
            gpiob.pb8.into_floating_input(&mut gpiob.crh), // rx
            gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh), // tx
            &mut afio.mapr,
        );
        can.set_bitrate(Bitrate::Br125kbps);
        can.enable_interrupts();
        can.enable_non_blocking();

        // Init CAN TX queue
        let (can_tx_producer, can_tx_consumer) = cx.local.q.split();

        // Init buttons
        let (_pa15, _pb3, pb4) = afio.mapr.disable_jtag(gpioa.pa15, gpiob.pb3, gpiob.pb4);
        let mut controller = Controller {
            button_ok: pb4.into_pull_up_input(&mut gpiob.crl),
            button_up: gpioa.pa0.into_pull_up_input(&mut gpioa.crl),
            button_down: gpioa.pa1.into_pull_up_input(&mut gpioa.crl),
            button_right: gpioa.pa2.into_pull_up_input(&mut gpioa.crl),
            button_left: gpioa.pa3.into_pull_up_input(&mut gpioa.crl),
        };
        controller.enable_interrupts(&mut afio, &mut cx.device.EXTI);

        // Init status LED
        let status_led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
        let status = CanaryStatus::Idle;
        blinker::spawn().unwrap();

        // Init SD Card
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
                    400.kHz(),
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

        (
            Shared {
                can,
                controller,
                can_tx_producer,
                status,
                volume_manager,
            },
            Local {
                can_tx_consumer,
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

    #[task(local = [status_led], shared = [status], priority = 1)]
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

    #[task(binds = USB_HP_CAN_TX, shared = [can], local = [can_tx_consumer])]
    fn can_sender(cx: can_sender::Context) {
        let mut can = cx.shared.can;
        let tx_queue = cx.local.can_tx_consumer;

        can.lock(|can| {
            can.bus.clear_tx_interrupt();

            if can.bus.is_transmitter_idle() {
                while let Some(frame) = tx_queue.peek() {
                    rprintln!("Attempting to transmit {:?}", frame);
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

    #[task(binds = USB_LP_CAN_RX0, shared = [can, can_tx_producer])]
    fn can_receiver(cx: can_receiver::Context) {
        let can = cx.shared.can;
        let tx_queue = cx.shared.can_tx_producer;

        (can, tx_queue).lock(|can, tx_queue| {
            while let Ok(frame) = can.bus.receive() {
                rprintln!("Received {:?}", frame);
                let _ = enqueue_frame(tx_queue, frame);
            }
        });
    }

    #[task(
        binds = EXTI4,
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
        shared = [controller],
        priority = 2
    )]
    fn clicked_ok(cx: clicked_ok::Context) {
        cx.shared.controller.button_ok.clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed OK");
    }

    #[task(
        binds = EXTI0,
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
        shared = [controller],
        priority = 2
    )]
    fn clicked_up(cx: clicked_up::Context) {
        cx.shared.controller.button_up.clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed UP");
    }

    #[task(
        binds = EXTI1,
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
        shared = [controller],
        priority = 2
    )]
    fn clicked_down(cx: clicked_down::Context) {
        cx.shared
            .controller
            .button_down
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed DOWN");
    }

    #[task(
        binds = EXTI2,
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
        shared = [controller],
        priority = 2
    )]
    fn clicked_right(cx: clicked_right::Context) {
        cx.shared
            .controller
            .button_right
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed RIGHT");
    }

    #[task(
        binds = EXTI3,
        local = [last_press_time: Option<Instant<u32, 1, TICK_RATE>> = None],
        shared = [controller],
        priority = 2
    )]
    fn clicked_left(cx: clicked_left::Context) {
        cx.shared
            .controller
            .button_left
            .clear_interrupt_pending_bit();
        if debounce_input(cx.local.last_press_time) {
            return;
        };

        rprintln!("Pressed LEFT");
    }
}
