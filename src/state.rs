use core::fmt::Debug;

use embedded_sdmmc::ShortFileName;
use heapless::Vec;
use rtt_target::rprintln;

use crate::{
    app::{MAX_SD_INDEX_AMOUNT, MAX_SD_INDEX_DEPTH},
    buttons::Button,
    can::{Bitrate, EmissionMode},
    render::*,
};

pub struct StateManager {
    display: Display,
    current_screen: Screen,
    pub state: State,
}

impl StateManager {
    // procedures communes à tous les écrans
    pub fn default_with_display(display: Display) -> Self {
        Self {
            display,
            current_screen: Screen::default(),
            state: State::default(),
        }
    }

    pub fn render(&mut self) {
        self.display.clear_buffer();
        match &self.current_screen {
            Screen::Home { selected_item } => draw_home(&mut self.display, selected_item),
            Screen::CaptureSelection { selected_index }
            | Screen::EmissionSelection { selected_index } => draw_file_selection(
                &mut self.display,
                self.state.dir_path.last(),
                if self.state.running {
                    &[]
                } else {
                    &self.state.dir_content
                },
                *selected_index,
            ),
            Screen::Emission => draw_emission(
                &mut self.display,
                self.state.dir_path.last().expect("a file was selected"),
                self.state.running,
                self.state.emission_count,
                &self.state.bitrate,
                &self.state.emission_mode,
                self.state.success_count,
            ),
            Screen::Capture => draw_capture(
                &mut self.display,
                self.state.dir_path.last(),
                self.state.running,
                &self.state.bitrate,
                self.state.capture_silent,
                self.state.success_count,
            ),
            Screen::EmissionSettings { selected_item } => draw_emission_settings(
                &mut self.display,
                selected_item,
                &self.state.bitrate,
                &self.state.emission_mode,
            ),
        }
        self.display.flush().unwrap();

        rprintln!("{:#?}", self);
    }

    pub fn press(&mut self, button: Button) {
        self.current_screen.press(button, &mut self.state);
        self.render();
    }

    pub fn current_screen(&self) -> &Screen {
        &self.current_screen
    }
}

impl Debug for StateManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{{
    current_screen: {:#?},
    state: {:#?},
}}",
            self.current_screen,
            State {
                dir_path: self.state.dir_path.clone(),
                dir_content: Vec::new(),
                ..self.state
            }
        )
    }
}

#[derive(Debug)]
pub enum Screen {
    Home { selected_item: HomeItem },
    EmissionSelection { selected_index: usize },
    Emission,
    EmissionSettings { selected_item: EmissionSettingsItem },
    CaptureSelection { selected_index: usize },
    Capture,
}

enum ScreenVariant {
    Home,
    EmissionSelection,
    Emission,
    EmissionSettings,
    CaptureSelection,
    Capture,
}

impl Screen {
    pub fn default() -> Self {
        Self::default_variant(ScreenVariant::Home)
    }

    fn default_variant(variant: ScreenVariant) -> Self {
        match variant {
            ScreenVariant::Home => Self::Home {
                selected_item: HomeItem::Emit,
            },
            ScreenVariant::EmissionSelection => Self::EmissionSelection { selected_index: 0 },
            ScreenVariant::Emission => Self::Emission,
            ScreenVariant::EmissionSettings => Self::EmissionSettings {
                selected_item: EmissionSettingsItem::Bitrate,
            },
            ScreenVariant::CaptureSelection => Self::CaptureSelection { selected_index: 0 },
            ScreenVariant::Capture => Self::Capture,
        }
    }

    pub fn press(&mut self, button: Button, state: &mut State) {
        match self {
            Self::Home { selected_item } => match button {
                Button::Ok => {
                    state.running = true;
                    *self = Self::default_variant(match selected_item {
                        HomeItem::Capture => ScreenVariant::CaptureSelection,
                        HomeItem::Emit => ScreenVariant::EmissionSelection,
                    })
                }
                Button::Right => *selected_item = HomeItem::Capture,
                Button::Left => *selected_item = HomeItem::Emit,
                _ => {}
            },
            Self::EmissionSelection { selected_index } => match button {
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
                        *self = Self::default_variant(ScreenVariant::Emission);
                    }
                },
                Button::Left => {
                    if state.dir_path.is_empty() {
                        state.soft_reset();
                        *self = Self::Home {
                            selected_item: HomeItem::Emit,
                        }
                    } else {
                        state.dir_path.pop();
                        *selected_index = 0;
                        state.running = true;
                    }
                }
            },
            Self::Emission => match (button, state.running) {
                (Button::Ok, _) => {
                    state.running = !state.running;
                    if state.running {
                        state.success_count = 0
                    }
                }
                (Button::Up, false) => {
                    state.emission_count = state.emission_count.saturating_add(1)
                }
                (Button::Down, false) => {
                    state.emission_count = state.emission_count.saturating_sub(1)
                }
                (Button::Right, false) => {
                    *self = Self::default_variant(ScreenVariant::EmissionSettings)
                }
                (Button::Left, false) => {
                    state.soft_reset();
                    *self = Self::Home {
                        selected_item: HomeItem::Emit,
                    }
                }
                _ => {}
            },
            Self::EmissionSettings { selected_item } => match button {
                Button::Ok => *self = Self::default_variant(ScreenVariant::Emission),
                Button::Up => selected_item.decrement(),
                Button::Down => selected_item.increment(),
                Button::Right => match selected_item {
                    EmissionSettingsItem::Bitrate => state.bitrate.increment(),
                    EmissionSettingsItem::Mode => state.emission_mode.increment(),
                },
                Button::Left => match selected_item {
                    EmissionSettingsItem::Bitrate => state.bitrate.decrement(),
                    EmissionSettingsItem::Mode => state.emission_mode.decrement(),
                },
            },
            Self::CaptureSelection { selected_index } => match button {
                Button::Ok => match &state.dir_content[*selected_index] {
                    (true, parent_dir) if parent_dir == &ShortFileName::parent_dir() => {
                        state.dir_path.pop();
                        *self = Self::default_variant(ScreenVariant::Capture);
                    }
                    (true, this_dir) if this_dir == &ShortFileName::this_dir() => {
                        *self = Self::default_variant(ScreenVariant::Capture);
                    }
                    (true, dir_name) => {
                        state.dir_path.push(dir_name.clone()).unwrap();
                        *self = Self::default_variant(ScreenVariant::Capture);
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
                    if state.dir_path.is_empty() {
                        state.soft_reset();
                        *self = Self::Home {
                            selected_item: HomeItem::Capture,
                        }
                    } else {
                        state.dir_path.pop();
                        *selected_index = 0;
                        state.running = true;
                    }
                }
            },
            Self::Capture => match (button, state.running) {
                (Button::Ok, _) => {
                    state.running = !state.running;
                    if state.running {
                        state.success_count = 0
                    }
                }
                (Button::Up, false) => state.bitrate.increment(),
                (Button::Down, false) => state.bitrate.decrement(),
                (Button::Right, false) => state.capture_silent = !state.capture_silent,
                (Button::Left, false) => {
                    state.soft_reset();
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
pub struct State {
    pub bitrate: Bitrate,
    pub emission_mode: EmissionMode,
    pub emission_count: u8,
    pub capture_silent: bool,
    pub running: bool,
    pub success_count: u32,
    pub dir_path: Vec<ShortFileName, MAX_SD_INDEX_DEPTH>,
    pub dir_content: Vec<(bool, ShortFileName), MAX_SD_INDEX_AMOUNT>,
}

impl State {
    pub fn default() -> Self {
        Self {
            bitrate: Bitrate::Br125kbps,
            emission_mode: EmissionMode::AwaitACK,
            emission_count: 1,
            capture_silent: false,
            running: false,
            success_count: 0,
            dir_path: Vec::new(),
            dir_content: Vec::new(),
        }
    }

    pub fn soft_reset(&mut self) {
        self.emission_count = 1;
        self.success_count = 0;
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
pub enum EmissionSettingsItem {
    Bitrate,
    Mode,
}

impl EmissionSettingsItem {
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
