//! CAN bus wrappers and relevant abstractions.

use bxcan::{filter::Mask32, Fifo, Frame};
use heapless::spsc::Producer;
use stm32f1xx_hal::{
    afio,
    can::Can,
    gpio::{Alternate, Pin},
    pac::{self, CAN1},
};

use crate::app::PCLK1_CLOCK_RATE_MHZ;

/// Structure wrapping a [`bxcan::Can<Can<CAN1>>`] and exposing a simplified API
pub struct CanContext {
    /// The wrapped [`bxcan::Can<Can<CAN1>>`] instance
    pub bus: bxcan::Can<Can<CAN1>>,
}

impl CanContext {
    /// Creates a new [`CanContext`] instance provided `rx` and `tx` pins
    ///
    /// The CAN bus will be initialized with an [`accept_all()`](Mask32::accept_all()) filter for
    /// the [`Fifo0`][Fifo::Fifo0] only. [`Fifo1`][Fifo::Fifo1] will not be used. The CAN bus is left
    /// disabled, enable it with [`enable_tx()`](CanContext::enable_tx()) or
    /// [`enable_rx()`](CanContext::enable_rx()).
    pub fn new(
        can_instance: Can<CAN1>,
        rx: Pin<'B', 8>,
        tx: Pin<'B', 9, Alternate>,
        mapr: &mut afio::MAPR,
    ) -> Self {
        can_instance.assign_pins((tx, rx), mapr);

        let mut can_bus = bxcan::Can::builder(can_instance).leave_disabled();
        can_bus
            .modify_filters()
            .enable_bank(0, Fifo::Fifo0, Mask32::accept_all());

        Self { bus: can_bus }
    }

    /// Enables the CAN bus in TX mode given a [`Bitrate`] and [`EmissionMode`].
    ///
    /// Only the [`TransmitMailboxEmpty`](bxcan::Interrupt::TransmitMailboxEmpty) interrupt will be
    /// enabled.
    pub fn enable_tx(&mut self, bitrate: Bitrate, mode: EmissionMode) {
        self.bus
            .modify_config()
            .set_bit_timing(bitrate.as_bit_timing())
            .set_automatic_retransmit(match mode {
                EmissionMode::AwaitACK => true,
                EmissionMode::IgnoreACK | EmissionMode::Loopback => false,
            })
            .set_loopback(match mode {
                EmissionMode::Loopback => true,
                EmissionMode::AwaitACK | EmissionMode::IgnoreACK => false,
            })
            .enable();
        self.bus
            .enable_interrupt(bxcan::Interrupt::TransmitMailboxEmpty);
    }

    /// Enables the CAN bus in RX mode given a [`Bitrate`] and `silent` flag.
    ///
    /// Only the [`Fifo0MessagePending`](bxcan::Interrupt::Fifo0MessagePending) interrupt will be
    /// enabled. If the `silent` flag is set, received frames will not be acknowledged.
    pub fn enable_rx(&mut self, bitrate: Bitrate, silent: bool) {
        self.bus
            .modify_config()
            .set_bit_timing(bitrate.as_bit_timing())
            .set_silent(silent)
            .enable();
        self.bus
            .enable_interrupt(bxcan::Interrupt::Fifo0MessagePending);
    }

    /// Disables the CAN bus.
    ///
    /// Both TX and RX will be deactivated. All frames queued in the [`Mailbox`](bxcan::Mailbox)es
    /// will be aborted.
    pub fn disable(&mut self) {
        self.bus.disable_interrupts(
            bxcan::Interrupts::FIFO0_MESSAGE_PENDING | bxcan::Interrupts::TRANSMIT_MAILBOX_EMPTY,
        );
        self.bus.abort(bxcan::Mailbox::Mailbox0);
        self.bus.abort(bxcan::Mailbox::Mailbox1);
        self.bus.abort(bxcan::Mailbox::Mailbox2);
    }
}

/// Enqueues `frame` in the provided `queue` and pends the
/// [`USB_HP_CAN_TX`][pac::Interrupt::USB_HP_CAN_TX] interrupt, allowing the frame to be sent
/// immediately.
pub fn enqueue_frame<const N: usize>(
    queue: &mut Producer<'_, Frame, N>,
    frame: Frame,
) -> Result<(), Frame> {
    queue.enqueue(frame)?;
    rtic::pend(pac::Interrupt::USB_HP_CAN_TX);
    Ok(())
}

/// A CAN bus bit rate
///
/// Available [`Bitrate`]s are the common bitrates defined by CANopen.
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum Bitrate {
    /// A bitrate of 1000 kbps
    Br1000kbps = 1_000_000,
    /// A bitrate of 800 kbps
    Br800kbps = 800_000,
    /// A bitrate of 500 kbps
    Br500kbps = 500_000,
    /// A bitrate of 250 kbps
    Br250kbps = 250_000,
    /// A bitrate of 125 kbps
    Br125kbps = 125_000,
    /// A bitrate of 100 kbps
    Br100kbps = 100_000,
    /// A bitrate of 83.333 kbps
    Br83kbps = 83_333,
    /// A bitrate of 50 kbps
    Br50kbps = 50_000,
    /// A bitrate of 20 kbps
    Br20kbps = 20_000,
    /// A bitrate of 10 kbps
    Br10kbps = 10_000,
}

impl Bitrate {
    /// A default [`Bitrate`] of 125 kbps.
    ///
    /// Used for [`State`][crate::state::State] initialisation.
    pub fn default() -> Self {
        Self::Br125kbps
    }

    /// Returns the bit timing corresponding to the [`Bitrate`].
    ///
    /// Bit timings are used to set the bit rate when enabling a [`CanContext`].
    ///
    /// Bit timings where calculated with <http://www.bittiming.can-wiki.info/> with a clock rate of
    /// 16 MHz (value of [`PCLK1_CLOCK_RATE_MHZ`]), a sample-point at 87.5% and a SJW of 1.
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

    /// Compile-time check for [`PCLK1_CLOCK_RATE_MHZ`] modification done without updating
    /// [`as_bit_timing()`][Bitrate::as_bit_timing()].
    const _HAS_PCLK1_CHANGED: () = assert!(
        PCLK1_CLOCK_RATE_MHZ == 16, // Change right hand side after updating bit timings
        "Updated PCLK1 rates without changing CAN bit timings"
    );

    /// Increments an instance to next valid [`Bitrate`].
    pub fn increment(&mut self) {
        match self {
            Self::Br1000kbps => *self = Self::Br1000kbps,
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

    /// Decrements an instance to next valid [`Bitrate`].
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
            Self::Br10kbps => *self = Self::Br10kbps,
        }
    }
}

/// A CAN bus emission mode
#[derive(Clone, Copy, Debug)]
pub enum EmissionMode {
    /// [Automatic retransmit](bxcan::CanConfig::set_automatic_retransmit) is enabled.
    ///
    /// Each frame will try to be sent until acknowledged by another node on the CAN bus.
    AwaitACK,
    /// [Automatic retransmit](bxcan::CanConfig::set_automatic_retransmit) is disabled.
    ///
    /// Each frame will only be sent once on the CAN bus, regardless of whether is was acknowledged
    /// or not.
    IgnoreACK,
    /// [Loopback](bxcan::CanConfig::set_loopback) is enabled.
    ///
    /// Each frame will only be sent once due to them being acknowledged by the device itself.
    ///
    /// This mode also allows to execute the frame sending logic without a CAN transceiver.
    Loopback,
}

impl EmissionMode {
    /// A default [`EmissionMode`]: an [`AwaitACK`][EmissionMode::AwaitACK].
    pub fn default() -> Self {
        Self::AwaitACK
    }

    /// Increments an instance to next [`EmissionMode`].
    ///
    /// Used for display and selection logic.
    pub fn increment(&mut self) {
        match self {
            Self::AwaitACK => *self = Self::IgnoreACK,
            Self::IgnoreACK => *self = Self::Loopback,
            Self::Loopback => *self = Self::AwaitACK,
        }
    }

    /// Decrements an instance to previous [`EmissionMode`].
    ///
    /// Used for display and selection logic.
    pub fn decrement(&mut self) {
        match self {
            Self::IgnoreACK => *self = Self::AwaitACK,
            Self::Loopback => *self = Self::IgnoreACK,
            Self::AwaitACK => *self = Self::Loopback,
        }
    }
}
