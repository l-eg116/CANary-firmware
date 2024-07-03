use core::fmt::Debug;

use rtt_target::rprintln;

use self::DisplayScreenVariant as DSV;
use crate::{
    buttons::ControllerButton,
    can::{Bitrate, EmissionMode},
};

pub struct DisplayManager {
    current_screen: DisplayScreen,
    settings: DisplayState,
}

impl DisplayManager {
    // procedures communes à tous les écrans
    pub fn default() -> DisplayManager {
        DisplayManager {
            current_screen: DisplayScreen::default(),
            settings: DisplayState::default(),
        }
    }
}

#[derive(Debug)]
enum DisplayScreen {
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

struct DisplayState {
    pub bitrate: Bitrate,
    pub emission_mode: EmissionMode,
    pub emission_count: u8,
    pub capture_silent: bool,
    pub running: bool,
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

    pub fn press(&mut self, button: ControllerButton, state: &mut DisplayState) {
        match self {
            Self::Home { selected_item } => match button {
                ControllerButton::Ok => {
                    *self = Self::default_variant(match selected_item {
                        HomeItem::Capture => DSV::CaptureFrameSelection,
                        HomeItem::Emit => DSV::EmissionFrameSelection,
                    })
                }
                ControllerButton::Right => *selected_item = HomeItem::Capture,
                ControllerButton::Left => *selected_item = HomeItem::Emit,
                _ => {}
            },
            Self::EmissionFrameSelection {} => match button {
                ControllerButton::Ok => *self = Self::default_variant(DSV::FrameEmission),
                ControllerButton::Up => todo!(),
                ControllerButton::Down => todo!(),
                ControllerButton::Right => todo!(),
                ControllerButton::Left => {
                    *self = Self::Home {
                        selected_item: HomeItem::Emit,
                    }
                }
            },
            Self::FrameEmission => match button {
                ControllerButton::Ok => state.running = !state.running,
                ControllerButton::Up => {
                    state.emission_count = state.emission_count.saturating_add(1)
                }
                ControllerButton::Down => {
                    state.emission_count = state.emission_count.saturating_sub(1)
                }
                ControllerButton::Right => {
                    *self = Self::default_variant(DSV::FrameEmissionSettings)
                }
                ControllerButton::Left => {
                    *self = Self::Home {
                        selected_item: HomeItem::Emit,
                    }
                }
            },
            Self::FrameEmissionSettings { selected_item } => match button {
                ControllerButton::Ok => *self = Self::default_variant(DSV::FrameEmission),
                ControllerButton::Up => selected_item.decrement(),
                ControllerButton::Down => selected_item.increment(),
                ControllerButton::Right => match selected_item {
                    FrameEmissionSettingsItems::Bitrate => state.bitrate.increment(),
                    FrameEmissionSettingsItems::Mode => state.emission_mode.increment(),
                },
                ControllerButton::Left => match selected_item {
                    FrameEmissionSettingsItems::Bitrate => state.bitrate.decrement(),
                    FrameEmissionSettingsItems::Mode => state.emission_mode.decrement(),
                },
            },
            Self::CaptureFrameSelection {} => match button {
                ControllerButton::Ok => *self = Self::default_variant(DSV::FrameCapture),
                ControllerButton::Up => todo!(),
                ControllerButton::Down => todo!(),
                ControllerButton::Right => todo!(),
                ControllerButton::Left => {
                    *self = Self::Home {
                        selected_item: HomeItem::Capture,
                    }
                }
            },
            Self::FrameCapture => match button {
                ControllerButton::Ok => state.running = !state.running,
                ControllerButton::Up => state.bitrate.increment(),
                ControllerButton::Down => state.bitrate.decrement(),
                ControllerButton::Right => state.capture_silent = !state.capture_silent,
                ControllerButton::Left => {
                    *self = Self::Home {
                        selected_item: HomeItem::Capture,
                    }
                }
            },
        }
    }
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
enum HomeItem {
    Emit,
    Capture,
}

#[derive(Debug)]
enum FrameEmissionSettingsItems {
    Bitrate,
    Mode,
}

impl FrameEmissionSettingsItems {
    pub fn increment(&mut self) {
        *self = match self {
            Self::Bitrate => Self::Mode,
            Self::Mode => Self::Mode,
        }
    }
    pub fn decrement(&mut self) {
        *self = match self {
            Self::Mode => Self::Bitrate,
            Self::Bitrate => Self::Bitrate,
        }
    }
}
