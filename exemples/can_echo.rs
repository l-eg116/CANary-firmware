#![no_std]
#![no_main]

use cortex_m::asm::nop;
use cortex_m_rt::entry;
use panic_rtt_target as _;

use bxcan::{filter::Mask32, Fifo, Frame, StandardId};
use nb::block;
use rtt_target::{rprintln, rtt_init_print};
use stm32f1xx_hal::{can::Can, pac, prelude::*};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    rprintln!("pac, flash & rcc");
    let dp = pac::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    // To meet CAN clock accuracy requirements an external crystal or ceramic
    // resonator must be used. The blue pill has a 8MHz external crystal.
    // Other boards might have a crystal with another frequency or none at all.
    rprintln!("clock");
    rcc.cfgr.use_hse(8.MHz()).freeze(&mut flash.acr);

    rprintln!("afio");
    let mut afio = dp.AFIO.constrain();

    rprintln!("can1");
    let mut can1 = {
        rprintln!("\tCan::new");
        let can = Can::new(dp.CAN1, dp.USB);

        rprintln!("\tGPIOA");
        let mut gpioa = dp.GPIOA.split();
        let rx = gpioa.pa11.into_floating_input(&mut gpioa.crh);
        let tx = gpioa.pa12.into_alternate_push_pull(&mut gpioa.crh);
        can.assign_pins((tx, rx), &mut afio.mapr);

        // APB1 (PCLK1): 8MHz, Bit rate: 125kBit/s, Sample Point 87.5%
        // Value was calculated with http://www.bittiming.can-wiki.info/
        rprintln!("\tCAN builder");
        bxcan::Can::builder(can)
            .set_bit_timing(0x001c_0003)
            // .set_loopback(true)
            .leave_disabled()
    };

    // Configure filters so that can frames can be received.
    rprintln!("Filters");
    let mut filters = can1.modify_filters();
    filters.enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

    // Drop filters to leave filter configuraiton mode.
    drop(filters);

    // Select the interface.
    let mut can = can1;

    // Split the peripheral into transmitter and receiver parts.
    rprintln!("Non blocking");
    block!(can.enable_non_blocking()).unwrap();

    // Get feedback LED
    let mut gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    rprintln!("Echoing...");
    loop {
        if let Ok(frame) = block!(can.receive()) {
            rprintln!("Echoed {:?}", frame);
            led.toggle();
            block!(can.transmit(&frame)).unwrap();
        }

        // for id in 0..10 {
        //     let frame = Frame::new_data(StandardId::new(id).unwrap(), [id as u8]);
        //     rprintln!("Transmiting {:?}", frame);
        //     block!(can.transmit(&frame)).unwrap();
        // }

        // for _ in 0..100_000 {
        //     nop();
        // }
        // led.toggle();
    }
}
