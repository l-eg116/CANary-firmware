#![no_std]
#![no_main]

use panic_halt as _;
use rtic::app;

#[app(device = stm32f1xx_hal::pac, peripherals = true)]
mod app {
    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        (Shared {}, Local {})
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}
