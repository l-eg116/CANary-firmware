#![no_std]
#![no_main]
#![allow(dead_code)]

use panic_halt as _;
use rtic::app;

mod can;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
mod app {
    use bxcan::Frame;
    use heapless::spsc::{Consumer, Producer, Queue};
    use stm32f1xx_hal::{can::Can, flash::FlashExt, prelude::*, rcc::RccExt};

    use crate::can::*;

    #[shared]
    struct Shared {
        can: CanContext,
    }

    #[local]
    struct Local {
        can_tx_producer: Producer<'static, Frame, CAN_TX_CAPACITY>,
        can_tx_consumer: Consumer<'static, Frame, CAN_TX_CAPACITY>,
    }

    const CAN_TX_CAPACITY: usize = 8;

    #[init(local = [q: Queue<Frame, CAN_TX_CAPACITY> = Queue::new()])]
    fn init(cx: init::Context) -> (Shared, Local) {
        // Init flash, RCC and clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();
        let _clocks = rcc.cfgr.use_hse(8.MHz()).freeze(&mut flash.acr);

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

        (
            Shared { can },
            Local {
                can_tx_producer,
                can_tx_consumer,
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(binds = USB_HP_CAN_TX, shared = [can], local = [can_tx_consumer])]
    fn can_sender(cx: can_sender::Context) {
        let mut can = cx.shared.can;
        let tx_queue = cx.local.can_tx_consumer;

        can.lock(|can| {
            can.bus.clear_tx_interrupt();

            if can.bus.is_transmitter_idle() {
                while let Some(frame) = tx_queue.dequeue() {
                    match can.bus.transmit(&frame) {
                        Ok(status) => assert_eq!(
                            status.dequeued_frame(),
                            None,
                            "All mailboxes should have been empty"
                        ),
                        Err(nb::Error::WouldBlock) => break,
                        Err(_) => unreachable!(),
                    }
                }
            }
        });
    }

    #[task(binds = USB_LP_CAN_RX0, shared = [can], local = [can_tx_producer])]
    fn can_receiver(cx: can_receiver::Context) {
        let mut can = cx.shared.can;
        let tx_queue = cx.local.can_tx_producer;

        can.lock(|can| {
            while let Ok(frame) = can.bus.receive() {
                let _ = enqueue_frame(tx_queue, frame);
            }
        });
    }
}
