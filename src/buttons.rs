use rtic_monotonics::{fugit::Instant, Monotonic as _};
use rtt_target::rprint;
use stm32f1xx_hal::{
    afio,
    gpio::{Edge, ExtiPin, Input, Pin, PullUp},
    pac::EXTI,
    timer::ExtU32 as _,
};

use crate::app::{Mono, DEBOUNCE_DELAY_MS, TICK_RATE};

/// Type alias for the OK button.
type OkButton = Pin<'B', 4, Input<PullUp>>;
/// Type alias for the UP button.
type UpButton = Pin<'A', 0, Input<PullUp>>;
/// Type alias for the DOWN button.
type DownButton = Pin<'A', 1, Input<PullUp>>;
/// Type alias for the RIGHT button.
type RightButton = Pin<'A', 2, Input<PullUp>>;
/// Type alias for the LEFT button.
type LeftButton = Pin<'A', 3, Input<PullUp>>;

/// Wrapper struct grouping buttons pins.
///
/// It exposes grouped calls for enabling or querying interrupts and others.
pub struct ButtonPanel {
    /// OK Button pin.
    pub button_ok: OkButton,
    /// UP Button pin.
    pub button_up: UpButton,
    /// DOWN Button pin.
    pub button_down: DownButton,
    /// RIGHT Button pin.
    pub button_right: RightButton,
    /// LEFT Button pin.
    pub button_left: LeftButton,
}

impl ButtonPanel {
    /// Sets all [ButtonPanel] buttons as interrupt sources.
    ///
    /// Buttons will trigger on the [Rising](Edge::Rising) edge, meaning when they get released
    /// (instead of when pressed) due to them being [PullUp] pins.
    pub fn enable_interrupts(&mut self, afio: &mut afio::Parts, exti: &mut EXTI) {
        self.button_ok.make_interrupt_source(afio);
        self.button_up.make_interrupt_source(afio);
        self.button_down.make_interrupt_source(afio);
        self.button_right.make_interrupt_source(afio);
        self.button_left.make_interrupt_source(afio);

        self.button_ok.trigger_on_edge(exti, Edge::Rising);
        self.button_up.trigger_on_edge(exti, Edge::Rising);
        self.button_down.trigger_on_edge(exti, Edge::Rising);
        self.button_right.trigger_on_edge(exti, Edge::Rising);
        self.button_left.trigger_on_edge(exti, Edge::Rising);

        self.button_ok.enable_interrupt(exti);
        self.button_up.enable_interrupt(exti);
        self.button_down.enable_interrupt(exti);
        self.button_right.enable_interrupt(exti);
        self.button_left.enable_interrupt(exti);
    }

    /// Clears all [ButtonPanel] buttons interrupt pending bits.
    pub fn clear_all_interrupt_pending_bits(&mut self) {
        self.button_ok.clear_interrupt_pending_bit();
        self.button_up.clear_interrupt_pending_bit();
        self.button_down.clear_interrupt_pending_bit();
        self.button_right.clear_interrupt_pending_bit();
        self.button_left.clear_interrupt_pending_bit();
    }

    /// Queries all [ButtonPanel] buttons interrupt state.
    pub fn get_interrupt_states(&self) -> ButtonPanelState {
        ButtonPanelState {
            ok_pressed: self.button_ok.check_interrupt(),
            up_pressed: self.button_up.check_interrupt(),
            down_pressed: self.button_down.check_interrupt(),
            right_pressed: self.button_right.check_interrupt(),
            left_pressed: self.button_left.check_interrupt(),
        }
    }

    /// Queries all [ButtonPanel] buttons state.
    pub fn get_states(&self) -> ButtonPanelState {
        ButtonPanelState {
            ok_pressed: self.button_ok.is_low(),
            up_pressed: self.button_up.is_low(),
            down_pressed: self.button_down.is_low(),
            right_pressed: self.button_right.is_low(),
            left_pressed: self.button_left.is_low(),
        }
    }
}

/// Enumeration of the buttons available on the hardware.
#[derive(Debug)]
pub enum Button {
    Ok,
    Up,
    Down,
    Right,
    Left,
}

/// Representation of the state of the buttons of a [ButtonPanel].
#[derive(Debug)]
pub struct ButtonPanelState {
    /// Whether the OK button is pressed.
    pub ok_pressed: bool,
    /// Whether the UP button is pressed.
    pub up_pressed: bool,
    /// Whether the DOWN button is pressed.
    pub down_pressed: bool,
    /// Whether the RIGHT button is pressed.
    pub right_pressed: bool,
    /// Whether the LEFT button is pressed.
    pub left_pressed: bool,
}

impl ButtonPanelState {
    /// A [ButtonPanelState] with all buttons released.
    pub fn default() -> Self {
        Self {
            ok_pressed: false,
            up_pressed: false,
            down_pressed: false,
            right_pressed: false,
            left_pressed: false,
        }
    }

    /// Debugging function printing to RTT the [ButtonPanelState].
    pub fn print(&self) {
        rprint!(
            "{} {} {} {} {}",
            if self.ok_pressed { "ok" } else { "--" },
            if self.up_pressed { "up" } else { "--" },
            if self.down_pressed { "down" } else { "----" },
            if self.right_pressed { "right" } else { "-----" },
            if self.left_pressed { "left" } else { "----" }
        )
    }
}

/// Updates an input [Instant] and returns whether the input should be ignored.
/// 
/// Physical switches can sometimes trigger multiple times when pressed or released. This function
/// aims to filter inputs judged too close (defined by [DEBOUNCE_DELAY_MS]).
/// 
/// This is done by updating the provided `last_press_time` and checking if the time delta is lower
/// than [DEBOUNCE_DELAY_MS]. The function then returns `true` if the input should be ignored.
pub fn debounce_input(last_press_time: &mut Option<Instant<u32, 1, TICK_RATE>>) -> bool {
    let now = Mono::now();
    let last_time = last_press_time
        .replace(now)
        .unwrap_or(Instant::<u32, 1, TICK_RATE>::from_ticks(0));

    // This operation can fail if Mono::now() overflows, which it will do after u32::MAX ~= 50 days
    now - last_time < DEBOUNCE_DELAY_MS.millis::<1, TICK_RATE>()
}
