use core::fmt::{Debug, Write};

use embedded_graphics::{
    geometry::Point,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    text::Text,
    Drawable,
};
use heapless::String;
use rtt_target::rprintln;
use ssd1306::{
    mode::BufferedGraphicsMode, prelude::I2CInterface, size::DisplaySize128x64, Ssd1306,
};
use stm32f1xx_hal::{
    gpio::{Alternate, OpenDrain, Pin},
    i2c::BlockingI2c,
    pac::I2C1,
};

use self::DisplayScreenVariant as DSV;
use crate::{
    buttons::Button,
    can::{Bitrate, EmissionMode},
};

pub type Display = Ssd1306<
    I2CInterface<
        BlockingI2c<
            I2C1,
            (
                Pin<'B', 6, Alternate<OpenDrain>>,
                Pin<'B', 7, Alternate<OpenDrain>>,
            ),
        >,
    >,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

pub struct DisplayManager {
    display: Display,
    current_screen: DisplayScreen,
    pub state: DisplayState,
}

impl DisplayManager {
    // procedures communes à tous les écrans
    pub fn default_with_display(display: Display) -> DisplayManager {
        DisplayManager {
            display,
            current_screen: DisplayScreen::default(),
            state: DisplayState::default(),
        }
    }

    pub fn render(&mut self) {
        let mut txt = String::<256>::new();
        txt.write_fmt(format_args!(
            "[{:?}]\n{:#?}",
            self.current_screen, self.state
        ))
        .unwrap();

        self.display.clear_buffer();
        Text::new(
            &txt,
            Point::new(0, 6),
            MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        )
        .draw(&mut self.display)
        .unwrap();
        self.display.flush().unwrap();

        rprintln!("{:#?}", self);
    }

    pub fn press(&mut self, button: Button) {
        self.current_screen.press(button, &mut self.state);
        self.render();
    }

    pub fn current_screen(&self) -> &DisplayScreen {
        &self.current_screen
    }
}

impl Debug for DisplayManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{{
    current_screen: {:#?},
    state: {:#?}\n
}}",
            self.current_screen, self.state
        )
    }
}

#[derive(Debug)]
pub enum DisplayScreen {
    Home {
        selected_item: HomeItem,
    },
    EmissionFrameSelection {},
    FrameEmission,
    FrameEmissionSettings {
        selected_item: FrameEmissionSettingsItems,
    },
    CaptureFrameSelection {},
    FrameCapture,
}

enum DisplayScreenVariant {
    Home,
    EmissionFrameSelection,
    FrameEmission,
    FrameEmissionSettings,
    CaptureFrameSelection,
    FrameCapture,
}

impl DisplayScreen {
    pub fn default() -> Self {
        Self::default_variant(DisplayScreenVariant::Home)
    }

    fn default_variant(variant: DisplayScreenVariant) -> Self {
        match variant {
            DSV::Home => Self::Home {
                selected_item: HomeItem::Emit,
            },
            DSV::EmissionFrameSelection => Self::EmissionFrameSelection {},
            DSV::FrameEmission => Self::FrameEmission,
            DSV::FrameEmissionSettings => Self::FrameEmissionSettings {
                selected_item: FrameEmissionSettingsItems::Bitrate,
            },
            DSV::CaptureFrameSelection => Self::CaptureFrameSelection {},
            DSV::FrameCapture => Self::FrameCapture,
        }
    }

    pub fn press(&mut self, button: Button, state: &mut DisplayState) {
        match self {
            Self::Home { selected_item } => match button {
                Button::Ok => {
                    *self = Self::default_variant(match selected_item {
                        HomeItem::Capture => DSV::CaptureFrameSelection,
                        HomeItem::Emit => DSV::EmissionFrameSelection,
                    })
                }
                Button::Right => *selected_item = HomeItem::Capture,
                Button::Left => *selected_item = HomeItem::Emit,
                _ => {}
            },
            Self::EmissionFrameSelection {} => match button {
                Button::Ok => *self = Self::default_variant(DSV::FrameEmission),
                Button::Up => todo!(),
                Button::Down => todo!(),
                Button::Right => todo!(),
                Button::Left => {
                    *self = Self::Home {
                        selected_item: HomeItem::Emit,
                    }
                }
            },
            Self::FrameEmission => match (button, state.running) {
                (Button::Ok, _) => state.running = !state.running,
                (Button::Up, false) => {
                    state.emission_count = state.emission_count.saturating_add(1)
                }
                (Button::Down, false) => {
                    state.emission_count = state.emission_count.saturating_sub(1)
                }
                (Button::Right, false) => *self = Self::default_variant(DSV::FrameEmissionSettings),
                (Button::Left, false) => {
                    *self = Self::Home {
                        selected_item: HomeItem::Emit,
                    }
                }
                _ => {}
            },
            Self::FrameEmissionSettings { selected_item } => match button {
                Button::Ok => *self = Self::default_variant(DSV::FrameEmission),
                Button::Up => selected_item.decrement(),
                Button::Down => selected_item.increment(),
                Button::Right => match selected_item {
                    FrameEmissionSettingsItems::Bitrate => state.bitrate.increment(),
                    FrameEmissionSettingsItems::Mode => state.emission_mode.increment(),
                },
                Button::Left => match selected_item {
                    FrameEmissionSettingsItems::Bitrate => state.bitrate.decrement(),
                    FrameEmissionSettingsItems::Mode => state.emission_mode.decrement(),
                },
            },
            Self::CaptureFrameSelection {} => match button {
                Button::Ok => *self = Self::default_variant(DSV::FrameCapture),
                Button::Up => todo!(),
                Button::Down => todo!(),
                Button::Right => todo!(),
                Button::Left => {
                    *self = Self::Home {
                        selected_item: HomeItem::Capture,
                    }
                }
            },
            Self::FrameCapture => match (button, state.running) {
                (Button::Ok, _) => state.running = !state.running,
                (Button::Up, false) => state.bitrate.increment(),
                (Button::Down, false) => state.bitrate.decrement(),
                (Button::Right, false) => state.capture_silent = !state.capture_silent,
                (Button::Left, false) => {
                    *self = Self::Home {
                        selected_item: HomeItem::Capture,
                    }
                }
                _ => {}
            },
        }
    }
}

#[derive(Debug)]
pub struct DisplayState {
    pub bitrate: Bitrate,
    pub emission_mode: EmissionMode,
    pub emission_count: u8,
    pub capture_silent: bool,
    pub running: bool,
}

impl DisplayState {
    pub fn default() -> Self {
        Self {
            bitrate: Bitrate::Br125kbps,
            emission_mode: EmissionMode::AwaitACK,
            emission_count: 1,
            capture_silent: false,
            running: false,
        }
    }
}

#[derive(Debug)]
pub enum HomeItem {
    Emit,
    Capture,
}

#[derive(Debug)]
pub enum FrameEmissionSettingsItems {
    Bitrate,
    Mode,
}

impl FrameEmissionSettingsItems {
    pub fn increment(&mut self) {
        *self = match self {
            Self::Bitrate | Self::Mode => Self::Mode,
        }
    }
    pub fn decrement(&mut self) {
        *self = match self {
            Self::Mode | Self::Bitrate => Self::Bitrate,
        }
    }
}
