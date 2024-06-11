#![no_std]
#![no_main]
#![allow(dead_code)]

use panic_halt as _;
use rtic::app;

mod can;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
mod app {
    use stm32f1xx_hal::{can::Can, flash::FlashExt, prelude::*, rcc::RccExt};

    use crate::{
        can::CanContext,
    };

    #[shared]
    struct Shared {
        can: CanContext,
    }

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        // Init flash, RCC and clocks
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();
        let _clocks = rcc.cfgr.use_hse(8.MHz()).freeze(&mut flash.acr);

        let mut gpiob = cx.device.GPIOB.split();
        let mut afio = cx.device.AFIO.constrain();

        // Init CAN bus
        let can = CanContext::new(
            Can::new(cx.device.CAN1, cx.device.USB),
            gpiob.pb8.into_floating_input(&mut gpiob.crh), // rx
            gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh), // tx
            &mut afio.mapr,
        );

        (
            Shared {
                can,
            },
            Local {},
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
