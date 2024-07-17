use core::fmt::{Debug, Write};

use embedded_graphics::{
    geometry::Point,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    text::Text,
    Drawable,
};
use embedded_sdmmc::ShortFileName;
use heapless::{String, Vec};
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
    app::{MAX_SD_INDEX_AMOUNT, MAX_SD_INDEX_DEPTH},
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
        let mut txt = String::<128>::new();
        txt.write_fmt(format_args!("{:#?}", self.current_screen))
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
    state: {:#?},
}}",
            self.current_screen,
            DisplayState {
                dir_path: self.state.dir_path.clone(),
                dir_content: Vec::new(),
                ..self.state
            }
        )
    }
}

#[derive(Debug)]
pub enum DisplayScreen {
    Home {
        selected_item: HomeItem,
    },
    EmissionFrameSelection {
        selected_index: usize,
    },
    FrameEmission,
    FrameEmissionSettings {
        selected_item: FrameEmissionSettingsItems,
    },
    CaptureFrameSelection {
        selected_index: usize,
    },
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
            DSV::EmissionFrameSelection => Self::EmissionFrameSelection { selected_index: 0 },
            DSV::FrameEmission => Self::FrameEmission,
            DSV::FrameEmissionSettings => Self::FrameEmissionSettings {
                selected_item: FrameEmissionSettingsItems::Bitrate,
            },
            DSV::CaptureFrameSelection => Self::CaptureFrameSelection { selected_index: 0 },
            DSV::FrameCapture => Self::FrameCapture,
        }
    }

    pub fn press(&mut self, button: Button, state: &mut DisplayState) {
        match self {
            Self::Home { selected_item } => match button {
                Button::Ok => {
                    state.running = true;
                    *self = Self::default_variant(match selected_item {
                        HomeItem::Capture => DSV::CaptureFrameSelection,
                        HomeItem::Emit => DSV::EmissionFrameSelection,
                    })
                }
                Button::Right => *selected_item = HomeItem::Capture,
                Button::Left => *selected_item = HomeItem::Emit,
                _ => {}
            },
            Self::EmissionFrameSelection { selected_index } => match button {
                Button::Up => {
                    *selected_index = selected_index.wrapping_sub(1);
                    if *selected_index >= state.dir_content.len() {
                        *selected_index = state.dir_content.len().saturating_sub(1);
                    }
                    rprintln!("selected {:?}", state.dir_content[*selected_index]);
                }
                Button::Down => {
                    *selected_index = selected_index.wrapping_add(1);
                    if *selected_index >= state.dir_content.len() {
                        *selected_index = 0;
                    }
                    rprintln!("selected {:?}", state.dir_content[*selected_index]);
                }
                Button::Right | Button::Ok => match &state.dir_content[*selected_index] {
                    (true, parent_dir) if parent_dir == &ShortFileName::parent_dir() => {
                        state.dir_path.pop();
                        *selected_index = 0;
                        state.running = true;
                    }
                    (true, this_dir) if this_dir == &ShortFileName::this_dir() => {
                        *selected_index = 0;
                        state.running = true;
                    }
                    (true, dir_name) => {
                        state.dir_path.push(dir_name.clone()).unwrap();
                        *selected_index = 0;
                        state.running = true;
                    }
                    (false, file_name) => {
                        state.dir_path.push(file_name.clone()).unwrap();
                        *self = Self::default_variant(DSV::FrameEmission);
                    }
                },
                Button::Left => {
                    state.clear_sd_index();
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
                    state.clear_sd_index();
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
            Self::CaptureFrameSelection { selected_index } => match button {
                Button::Ok => match &state.dir_content[*selected_index] {
                    (true, parent_dir) if parent_dir == &ShortFileName::parent_dir() => {
                        state.dir_path.pop();
                        *self = Self::default_variant(DSV::FrameCapture);
                    }
                    (true, this_dir) if this_dir == &ShortFileName::this_dir() => {
                        *self = Self::default_variant(DSV::FrameCapture);
                    }
                    (true, dir_name) => {
                        state.dir_path.push(dir_name.clone()).unwrap();
                        *self = Self::default_variant(DSV::FrameCapture);
                    }
                    (false, _) => unreachable!(),
                },
                Button::Up => {
                    *selected_index = selected_index.wrapping_sub(1);
                    if *selected_index >= state.dir_content.len() {
                        *selected_index = state.dir_content.len().saturating_sub(1);
                    }
                    rprintln!("selected {:?}", state.dir_content[*selected_index]);
                }
                Button::Down => {
                    *selected_index = selected_index.wrapping_add(1);
                    if *selected_index >= state.dir_content.len() {
                        *selected_index = 0;
                    }
                    rprintln!("selected {:?}", state.dir_content[*selected_index]);
                }
                Button::Right => match &state.dir_content[*selected_index] {
                    (true, parent_dir) if parent_dir == &ShortFileName::parent_dir() => {
                        state.dir_path.pop();
                        *selected_index = 0;
                        state.running = true;
                    }
                    (true, this_dir) if this_dir == &ShortFileName::this_dir() => {
                        *selected_index = 0;
                        state.running = true;
                    }
                    (true, dir_name) => {
                        state.dir_path.push(dir_name.clone()).unwrap();
                        *selected_index = 0;
                        state.running = true;
                    }
                    (false, _) => unreachable!(),
                },
                Button::Left => {
                    state.clear_sd_index();
                    *self = Self::Home {
                        selected_item: HomeItem::Emit,
                    }
                }
            },
            Self::FrameCapture => match (button, state.running) {
                (Button::Ok, _) => state.running = !state.running,
                (Button::Up, false) => state.bitrate.increment(),
                (Button::Down, false) => state.bitrate.decrement(),
                (Button::Right, false) => state.capture_silent = !state.capture_silent,
                (Button::Left, false) => {
                    state.clear_sd_index();
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
    pub dir_path: Vec<ShortFileName, MAX_SD_INDEX_DEPTH>,
    pub dir_content: Vec<(bool, ShortFileName), MAX_SD_INDEX_AMOUNT>,
}

impl DisplayState {
    pub fn default() -> Self {
        Self {
            bitrate: Bitrate::Br125kbps,
            emission_mode: EmissionMode::AwaitACK,
            emission_count: 1,
            capture_silent: false,
            running: false,
            dir_path: Vec::new(),
            dir_content: Vec::new(),
        }
    }

    pub fn clear_sd_index(&mut self) {
        self.dir_path = Vec::new();
        self.dir_content = Vec::new();
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
