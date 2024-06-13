#![no_std]
#![no_main]
#![allow(dead_code)]

use cortex_m::asm::nop;
use panic_rtt_target as _;
use rtic::app;

mod buttons;
mod can;

fn bootleg_delay(n: usize) {
    for _ in 0..n {
        nop();
    }
}

#[app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [TIM2])]
mod app {
    use bxcan::Frame;
    use heapless::spsc::{Consumer, Producer, Queue};
    use rtic_monotonics::systick::prelude::*;
    use rtt_target::{debug_rtt_init_print, rprintln};
    use stm32f1xx_hal::{can::Can, flash::FlashExt, gpio::ExtiPin, prelude::*, rcc::RccExt};

    use crate::bootleg_delay;
    use crate::buttons::*;
    use crate::can::*;

    const CAN_TX_CAPACITY: usize = 8;
    systick_monotonic!(Mono, 1_000);

    #[shared]
    struct Shared {
        can: CanContext,
        #[lock_free]
        controller: Controller,
        can_tx_producer: Producer<'static, Frame, CAN_TX_CAPACITY>,
    }

    #[local]
    struct Local {
        can_tx_consumer: Consumer<'static, Frame, CAN_TX_CAPACITY>,
    }

    #[init(local = [q: Queue<Frame, CAN_TX_CAPACITY> = Queue::new()])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        debug_rtt_init_print!();
        rprintln!("Initializing...");

        // Init flash, RCC and clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();
        let _clocks = rcc.cfgr.use_hse(8.MHz()).freeze(&mut flash.acr);

        Mono::start(cx.core.SYST, 8_000_000);

        let mut gpioa = cx.device.GPIOA.split();
        let mut gpiob = cx.device.GPIOB.split();
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

        (
            Shared {
                can,
                controller,
                can_tx_producer,
            },
            Local { can_tx_consumer },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        rprintln!("Entering idle loop");
        loop {
            // Systick::delay(100.millis()).await;
            // rprintln!("Idling...");
        }
    }

    #[task(priority = 1)]
    async fn idler(_cx: idler::Context) {
        loop {
            Mono::delay(500.millis()).await;
            rprintln!("Idling...");
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

    #[task(binds = EXTI4, shared = [controller], priority = 2)]
    fn clicked_ok(cx: clicked_ok::Context) {
        let controller = cx.shared.controller;

        bootleg_delay(100);
        controller.button_ok.clear_interrupt_pending_bit();
        rprintln!("Pressed OK");
    }

    #[task(binds = EXTI0, shared = [controller], priority = 2)]
    fn clicked_up(cx: clicked_up::Context) {
        let controller = cx.shared.controller;

        bootleg_delay(100);
        controller.button_up.clear_interrupt_pending_bit();
        rprintln!("Pressed UP");
    }

    #[task(binds = EXTI1, shared = [controller], priority = 2)]
    fn clicked_down(cx: clicked_down::Context) {
        let controller = cx.shared.controller;

        bootleg_delay(100);
        controller.button_down.clear_interrupt_pending_bit();
        rprintln!("Pressed DOWN");
    }

    #[task(binds = EXTI2, shared = [controller], priority = 2)]
    fn clicked_right(cx: clicked_right::Context) {
        let controller = cx.shared.controller;

        bootleg_delay(100);
        controller.button_right.clear_interrupt_pending_bit();
        rprintln!("Pressed RIGHT");
    }

    #[task(binds = EXTI3, shared = [controller], priority = 2)]
    fn clicked_left(cx: clicked_left::Context) {
        let controller = cx.shared.controller;

        bootleg_delay(100);
        controller.button_left.clear_interrupt_pending_bit();
        rprintln!("Pressed LEFT");
    }
}
