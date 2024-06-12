use rtt_target::rprint;
use stm32f1xx_hal::{
    afio,
    gpio::{Edge, ExtiPin, Input, Pin, PullUp},
    pac::EXTI,
};

type OkButton = Pin<'B', 4, Input<PullUp>>;
type UpButton = Pin<'A', 0, Input<PullUp>>;
type DownButton = Pin<'A', 1, Input<PullUp>>;
type RightButton = Pin<'A', 2, Input<PullUp>>;
type LeftButton = Pin<'A', 3, Input<PullUp>>;

pub struct Controller {
    pub button_ok: OkButton,
    pub button_up: UpButton,
    pub button_down: DownButton,
    pub button_right: RightButton,
    pub button_left: LeftButton,
}

impl Controller {
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

    pub fn clear_all_interrupt_pending_bits(&mut self) {
        self.button_ok.clear_interrupt_pending_bit();
        self.button_up.clear_interrupt_pending_bit();
        self.button_down.clear_interrupt_pending_bit();
        self.button_right.clear_interrupt_pending_bit();
        self.button_left.clear_interrupt_pending_bit();
    }

    pub fn get_interupt_states(&self) -> ControllerState {
        ControllerState {
            ok_pressed: self.button_ok.check_interrupt(),
            up_pressed: self.button_up.check_interrupt(),
            down_pressed: self.button_down.check_interrupt(),
            right_pressed: self.button_right.check_interrupt(),
            left_pressed: self.button_left.check_interrupt(),
        }
    }

    pub fn get_states(&self) -> ControllerState {
        ControllerState {
            ok_pressed: self.button_ok.is_low(),
            up_pressed: self.button_up.is_low(),
            down_pressed: self.button_down.is_low(),
            right_pressed: self.button_right.is_low(),
            left_pressed: self.button_left.is_low(),
        }
    }
}

#[derive(Debug)]
pub enum ControllerButton {
    Ok,
    Up,
    Down,
    Right,
    Left,
}

#[derive(Debug)]
pub struct ControllerState {
    pub ok_pressed: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub right_pressed: bool,
    pub left_pressed: bool,
}

impl ControllerState {
    pub fn default() -> ControllerState {
        ControllerState {
            ok_pressed: false,
            up_pressed: false,
            down_pressed: false,
            right_pressed: false,
            left_pressed: false,
        }
    }

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
