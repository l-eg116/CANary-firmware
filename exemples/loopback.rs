//! Showcases advanced CAN filter capabilities.
//! Does not require additional transceiver hardware.

#![no_main]
#![no_std]

use bxcan::{
    filter::{ListEntry16, ListEntry32, Mask16},
    ExtendedId, Fifo, Frame, StandardId,
};
// use panic_halt as _;
use panic_rtt_target as _;

use cortex_m_rt::entry;
use nb::block;
use rtt_target::{rprintln, rtt_init_print};
use stm32f1xx_hal::{can::Can, pac, prelude::*};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    rprintln!("Taking peripherals");
    let dp = pac::Peripherals::take().unwrap();

    rprintln!("Contraining things");
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();

    // To meet CAN clock accuracy requirements, an external crystal or ceramic
    // resonator must be used.
    rprintln!("Clocks");
    rcc.cfgr.use_hse(8.MHz()).freeze(&mut flash.acr);

    rprintln!("Can::new");
    #[cfg(not(feature = "connectivity"))]
    let can = Can::new(dp.CAN1, dp.USB);

    #[cfg(feature = "connectivity")]
    let can = Can::new(dp.CAN1);

    // Use loopback mode: No pins need to be assigned to peripheral.
    // APB1 (PCLK1): 8MHz, Bit rate: 500Bit/s, Sample Point 87.5%
    // Value was calculated with http://www.bittiming.can-wiki.info/
    rprintln!("bxcan::Can::builder");
    let mut can = bxcan::Can::builder(can)
        .set_bit_timing(0x001c_0000)
        .set_loopback(true)
        .set_silent(true)
        .leave_disabled();

    rprintln!("filters");
    let mut filters = can.modify_filters();
    assert!(filters.num_banks() > 3);

    // The order of the added filters is important: it must match configuration
    // of the `split_filters_advanced()` method.

    // 2x 11bit id + mask filter bank: Matches 0, 1, 2
    // TODO: Make this accept also ID 2
    rprintln!("filters.enable_bank 0");
    filters.enable_bank(
        0,
        Fifo::Fifo0,
        [
            // accepts 0 and 1
            Mask16::frames_with_std_id(StandardId::new(0).unwrap(), StandardId::new(1).unwrap()),
            // accepts 0 and 2
            Mask16::frames_with_std_id(StandardId::new(0).unwrap(), StandardId::new(2).unwrap()),
        ],
    );

    // 2x 29bit id filter bank: Matches 4, 5
    rprintln!("filters.enable_bank 1");
    filters.enable_bank(
        1,
        Fifo::Fifo0,
        [
            ListEntry32::data_frames_with_id(ExtendedId::new(4).unwrap()),
            ListEntry32::data_frames_with_id(ExtendedId::new(5).unwrap()),
        ],
    );

    // 4x 11bit id filter bank: Matches 8, 9, 10, 11
    rprintln!("filters.enable_bank 2");
    filters.enable_bank(
        2,
        Fifo::Fifo0,
        [
            ListEntry16::data_frames_with_id(StandardId::new(8).unwrap()),
            ListEntry16::data_frames_with_id(StandardId::new(9).unwrap()),
            ListEntry16::data_frames_with_id(StandardId::new(10).unwrap()),
            ListEntry16::data_frames_with_id(StandardId::new(11).unwrap()),
        ],
    );

    // Enable filters.
    rprintln!("drop filters");
    drop(filters);
    
    // Sync to the bus and start normal operation.
    rprintln!("enable non blocking");
    block!(can.enable_non_blocking()).ok();
    
    // Some messages shall pass the filters.
    rprintln!("passing messages");
    for &id in &[0, 1, 2, 8, 9, 10, 11] {
        let frame_tx = Frame::new_data(StandardId::new(id).unwrap(), [id as u8]);
        block!(can.transmit(&frame_tx)).unwrap();
        let frame_rx = block!(can.receive()).unwrap();
        assert_eq!(frame_tx, frame_rx);
    }
    rprintln!("more passing");
    for &id in &[4, 5] {
        let frame_tx = Frame::new_data(ExtendedId::new(id).unwrap(), [id as u8]);
        block!(can.transmit(&frame_tx)).unwrap();
        let frame_rx = block!(can.receive()).unwrap();
        assert_eq!(frame_tx, frame_rx);
    }

    // Some messages shall not be received.
    rprintln!("not received messages");
    for &id in &[3, 6, 7, 12] {
        let frame_tx = Frame::new_data(ExtendedId::new(id).unwrap(), [id as u8]);
        block!(can.transmit(&frame_tx)).unwrap();
        while !can.is_transmitter_idle() {}

        assert!(can.receive().is_err());
    }

    rprintln!("success !");
    let mut gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_low();

    loop {}
}
