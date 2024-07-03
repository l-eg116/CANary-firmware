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
    tx_mode: EmissionMode,
    pub bus: bxcan::Can<Can<CAN1>>,
}

impl CanContext {
    pub fn new(
        can_instance: Can<CAN1>,
        rx: Pin<'B', 8>,
        tx: Pin<'B', 9, Alternate>,
        mapr: &mut afio::MAPR,
    ) -> Self {
        can_instance.assign_pins((tx, rx), mapr);

        let mut can_bus = bxcan::Can::builder(can_instance)
            .set_bit_timing(Bitrate::default().as_bit_timing())
            .leave_disabled();
        can_bus
            .modify_filters()
            .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

        Self {
            bitrate: Bitrate::default(),
            tx_mode: EmissionMode::default(),
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

    pub fn tx_mode(&self) -> EmissionMode {
        self.tx_mode
    }

    pub fn set_tx_mode(&mut self, tx_mode: EmissionMode) {
        let bus_config = self.bus.modify_config();
        self.tx_mode = tx_mode;
        match self.tx_mode {
            EmissionMode::AwaitACK => bus_config // Expects some answer on the bus
                .set_automatic_retransmit(true)
                .set_loopback(false),
            EmissionMode::IgnoreACK => bus_config // Expects no answers on the bus
                .set_automatic_retransmit(false)
                .set_loopback(false),
            EmissionMode::Loopback => bus_config // Expects no bus
                .set_automatic_retransmit(true)
                .set_loopback(true),
        }
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

pub fn enqueue_frame<const N: usize>(
    queue: &mut Producer<'_, Frame, N>,
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
    pub fn default() -> Self {
        Self::Br125kbps
    }

    // Bit timings calculated with http://www.bittiming.can-wiki.info/
    // Bit timings are calculated for PCLK1 = 16MHz
    fn as_bit_timing(&self) -> u32 {
        match self {
            Self::Br1000kbps => 0x001c0000,
            Self::Br800kbps => 0x00070001,
            Self::Br500kbps => 0x001c0001,
            Self::Br250kbps => 0x001c0003,
            Self::Br125kbps => 0x001c0007,
            Self::Br100kbps => 0x001c0009,
            Self::Br83kbps => 0x001c000b,
            Self::Br50kbps => 0x001c0013,
            Self::Br20kbps => 0x001c0031,
            Self::Br10kbps => 0x001c0063,
        }
    }

    fn as_u32(&self) -> u32 {
        match self {
            Self::Br1000kbps => 1_000_000,
            Self::Br800kbps => 800_000,
            Self::Br500kbps => 500_000,
            Self::Br250kbps => 250_000,
            Self::Br125kbps => 125_000,
            Self::Br100kbps => 100_000,
            Self::Br83kbps => 83_333,
            Self::Br50kbps => 50_000,
            Self::Br20kbps => 20_000,
            Self::Br10kbps => 10_000,
        }
    }

    pub fn increment(&mut self) {
        match self {
            Self::Br1000kbps => *self = Self::Br10kbps,
            Self::Br800kbps => *self = Self::Br1000kbps,
            Self::Br500kbps => *self = Self::Br800kbps,
            Self::Br250kbps => *self = Self::Br500kbps,
            Self::Br125kbps => *self = Self::Br250kbps,
            Self::Br100kbps => *self = Self::Br125kbps,
            Self::Br83kbps => *self = Self::Br100kbps,
            Self::Br50kbps => *self = Self::Br83kbps,
            Self::Br20kbps => *self = Self::Br50kbps,
            Self::Br10kbps => *self = Self::Br20kbps,
        }
    }

    pub fn decrement(&mut self) {
        match self {
            Self::Br1000kbps => *self = Self::Br800kbps,
            Self::Br800kbps => *self = Self::Br500kbps,
            Self::Br500kbps => *self = Self::Br250kbps,
            Self::Br250kbps => *self = Self::Br125kbps,
            Self::Br125kbps => *self = Self::Br100kbps,
            Self::Br100kbps => *self = Self::Br83kbps,
            Self::Br83kbps => *self = Self::Br50kbps,
            Self::Br50kbps => *self = Self::Br20kbps,
            Self::Br20kbps => *self = Self::Br10kbps,
            Self::Br10kbps => *self = Self::Br1000kbps,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum EmissionMode {
    AwaitACK,
    IgnoreACK,
    Loopback,
}

impl EmissionMode {
    pub fn default() -> Self {
        Self::AwaitACK
    }

    pub fn increment(&mut self) {
        match self {
            Self::AwaitACK => *self = Self::IgnoreACK,
            Self::IgnoreACK => *self = Self::Loopback,
            Self::Loopback => *self = Self::AwaitACK,
        }
    }

    pub fn decrement(&mut self) {
        match self {
            Self::IgnoreACK => *self = Self::AwaitACK,
            Self::Loopback => *self = Self::IgnoreACK,
            Self::AwaitACK => *self = Self::Loopback,
        }
    }
}
