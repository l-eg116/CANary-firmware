use bxcan::{filter::Mask32, Fifo, Frame};
use heapless::spsc::Producer;
use nb::block;
use stm32f1xx_hal::{
    afio,
    can::Can,
    gpio::{Alternate, Pin},
    pac::{self, CAN1},
};

pub struct CanContext {
    bitrate: Bitrate,
    pub bus: bxcan::Can<Can<CAN1>>,
}

impl CanContext {
    pub fn new(
        can_instance: Can<CAN1>,
        rx: Pin<'B', 8>,
        tx: Pin<'B', 9, Alternate>,
        mapr: &mut afio::MAPR,
    ) -> CanContext {
        can_instance.assign_pins((tx, rx), mapr);

        let mut can_bus = bxcan::Can::builder(can_instance)
            .set_bit_timing(Bitrate::default().as_bit_timing())
            .leave_disabled();
        can_bus
            .modify_filters()
            .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

        CanContext {
            bitrate: Bitrate::default(),
            bus: can_bus,
        }
    }

    pub fn bitrate(&self) -> Bitrate {
        self.bitrate
    }

    pub fn set_bitrate(&mut self, bitrate: Bitrate) {
        self.bitrate = bitrate;

        self.bus
            .modify_config()
            .set_bit_timing(self.bitrate.as_bit_timing())
            .leave_disabled();
    }

    pub fn enable_non_blocking(&mut self) {
        block!(self.bus.enable_non_blocking()).unwrap();
    }

    pub fn enable_interrupts(&mut self) {
        self.bus.enable_interrupts(
            bxcan::Interrupts::FIFO0_MESSAGE_PENDING | bxcan::Interrupts::TRANSMIT_MAILBOX_EMPTY,
        );
    }

    pub fn disable_interrupts(&mut self) {
        self.bus.disable_interrupts(
            bxcan::Interrupts::FIFO0_MESSAGE_PENDING | bxcan::Interrupts::TRANSMIT_MAILBOX_EMPTY,
        );
    }
}

pub fn enqueue_frame<'a, const N: usize>(
    queue: &mut Producer<'a, Frame, N>,
    frame: Frame,
) -> Result<(), Frame> {
    queue.enqueue(frame)?;
    rtic::pend(pac::Interrupt::USB_HP_CAN_TX);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub enum Bitrate {
    Br1000kbps,
    Br800kbps,
    Br500kbps,
    Br250kbps,
    Br125kbps,
    Br100kbps,
    Br83kbps,
    Br50kbps,
    Br20kbps,
    Br10kbps,
}

impl Bitrate {
    pub fn default() -> Bitrate {
        Bitrate::Br125kbps
    }

    // Bit timings calculated with http://www.bittiming.can-wiki.info/
    // Bit timings are calculated for PCLK1 = 16MHz
    fn as_bit_timing(&self) -> u32 {
        match self {
            Bitrate::Br1000kbps => 0x001c0000,
            Bitrate::Br800kbps => 0x00070001,
            Bitrate::Br500kbps => 0x001c0001,
            Bitrate::Br250kbps => 0x001c0003,
            Bitrate::Br125kbps => 0x001c0007,
            Bitrate::Br100kbps => 0x001c0009,
            Bitrate::Br83kbps => 0x001c000b,
            Bitrate::Br50kbps => 0x001c0013,
            Bitrate::Br20kbps => 0x001c0031,
            Bitrate::Br10kbps => 0x001c0063,
        }
    }

    fn as_u32(&self) -> u32 {
        match self {
            Bitrate::Br1000kbps => 1_000_000,
            Bitrate::Br800kbps => 800_000,
            Bitrate::Br500kbps => 500_000,
            Bitrate::Br250kbps => 250_000,
            Bitrate::Br125kbps => 125_000,
            Bitrate::Br100kbps => 100_000,
            Bitrate::Br83kbps => 83_333,
            Bitrate::Br50kbps => 50_000,
            Bitrate::Br20kbps => 20_000,
            Bitrate::Br10kbps => 10_000,
        }
    }

    pub fn increment(&mut self) {
        match self {
            Bitrate::Br1000kbps => *self = Bitrate::Br10kbps,
            Bitrate::Br800kbps => *self = Bitrate::Br1000kbps,
            Bitrate::Br500kbps => *self = Bitrate::Br800kbps,
            Bitrate::Br250kbps => *self = Bitrate::Br500kbps,
            Bitrate::Br125kbps => *self = Bitrate::Br250kbps,
            Bitrate::Br100kbps => *self = Bitrate::Br125kbps,
            Bitrate::Br83kbps => *self = Bitrate::Br100kbps,
            Bitrate::Br50kbps => *self = Bitrate::Br83kbps,
            Bitrate::Br20kbps => *self = Bitrate::Br50kbps,
            Bitrate::Br10kbps => *self = Bitrate::Br20kbps,
        }
    }

    pub fn decrement(&mut self) {
        match self {
            Bitrate::Br1000kbps => *self = Bitrate::Br800kbps,
            Bitrate::Br800kbps => *self = Bitrate::Br500kbps,
            Bitrate::Br500kbps => *self = Bitrate::Br250kbps,
            Bitrate::Br250kbps => *self = Bitrate::Br125kbps,
            Bitrate::Br125kbps => *self = Bitrate::Br100kbps,
            Bitrate::Br100kbps => *self = Bitrate::Br83kbps,
            Bitrate::Br83kbps => *self = Bitrate::Br50kbps,
            Bitrate::Br50kbps => *self = Bitrate::Br20kbps,
            Bitrate::Br20kbps => *self = Bitrate::Br10kbps,
            Bitrate::Br10kbps => *self = Bitrate::Br1000kbps,
        }
    }
}
