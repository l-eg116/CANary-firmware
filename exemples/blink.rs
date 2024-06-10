//! Showcases advanced CAN filter capabilities.
//! Does not require additional transceiver hardware.

#![no_main]
#![no_std]

use cortex_m::asm::nop;
use panic_rtt_target as _;

use cortex_m_rt::entry;
use rtt_target::{rprintln, rtt_init_print};
use stm32f1xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    rprintln!("Taking peripherals");
    let dp = pac::Peripherals::take().unwrap();

    rprintln!("Contraining things");
    let mut _flash = dp.FLASH.constrain();
    let _rcc = dp.RCC.constrain();

    let mut gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    let mut toggle = true;

    loop {
        for _ in 0..100_000 {
            nop();
        }
        
        if toggle {
            led.set_high();
            rprintln!("high");
        } else {
            led.set_low();
            rprintln!("low");
        }

        toggle = !toggle;
    }
}
