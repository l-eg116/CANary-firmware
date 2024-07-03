use core::fmt::Debug;

use rtt_target::rprintln;

use crate::can::{Bitrate, EmissionMode};

struct DisplayManager<S: DisplayScreen> {
    pub current_screen: S,
    pub bitrate: Bitrate,
    pub mode: EmissionMode,
}

impl<S: DisplayScreen> DisplayManager<S> {
    // procedures communes à tous les écrans
    pub fn default() -> DisplayManager<Home> {
        DisplayManager {
            current_screen: Home::default(),
            bitrate: Bitrate::default(),
            mode: EmissionMode::default(),
        }
    }
}

trait DisplayScreen: Debug {
    fn render(&self) {
        rprintln!("{self:?}");
    }
}

// ############
// ### HOME ###
// ############
#[derive(Debug)]
struct Home {
    pub selected_item: HomeItems,
}
impl DisplayScreen for Home {}

#[derive(Debug)]
enum HomeItems {
    Emit,
    Capture,
} //noms non contractuels

impl Home {
    pub fn default() -> Self {
        Self {
            selected_item: HomeItems::Emit,
        }
    }
}

impl DisplayManager<Home> {
    pub fn right_pressed(&mut self) {
        self.current_screen.selected_item = match self.current_screen.selected_item {
            HomeItems::Emit => HomeItems::Capture,
            HomeItems::Capture => HomeItems::Emit,
        }
    }

    pub fn left_pressed(&mut self) {
        self.right_pressed()
    }

    pub fn ok_pressed(&mut self) {
        match self.current_screen.selected_item {
            HomeItems::Emit => todo!("GOTO EmissionFrameSelection"),
            HomeItems::Capture => todo!("GOTO CaptureFileSelection"),
        }
    }
}

// ##############################
// ### EMISSIONFRAMESELECTION ###
// ##############################
#[derive(Debug)]
struct EmissionFrameSelection {
    // pub selected_item: EmissionFrameSelectionItems
}
impl DisplayScreen for EmissionFrameSelection {}

impl DisplayManager<EmissionFrameSelection> {
    // procedures relatives à l'écran EmissionFrameSelection
}

// #####################
// ### FRAMEEMISSION ###
// #####################
#[derive(Debug)]
struct FrameEmission {
    pub frame_count: u8,
    pub is_sending_frames: bool,
}
impl DisplayScreen for FrameEmission {}

impl FrameEmission {
    pub fn default() -> Self {
        Self {
            frame_count: 1,
            is_sending_frames: false,
        }
    }
}

impl DisplayManager<FrameEmission> {
    pub fn up_pressed(&mut self) {
        self.current_screen.frame_count = self.current_screen.frame_count.saturating_add(1);
    }

    pub fn down_pressed(&mut self) {
        self.current_screen.frame_count = self.current_screen.frame_count.saturating_sub(1);
    }

    pub fn right_pressed(&mut self) {
        todo!("GOTO FrameEmissionSettings")
    }

    pub fn left_pressed(&mut self) {
        todo!("GOTO EmissionFrameSelection OR Home ?")
    }

    pub fn ok_pressed(&mut self) {
        self.current_screen.is_sending_frames = !self.current_screen.is_sending_frames;
    }
}

// #############################
// ### FRAMEEMISSIONSETTINGS ###
// #############################
#[derive(Debug)]
struct FrameEmissionSettings {
    pub selected_item: FrameEmissionSettingsItems,
}
impl DisplayScreen for FrameEmissionSettings {}
#[derive(Debug)]
enum FrameEmissionSettingsItems {
    Bitrate,
    Mode,
}

impl FrameEmissionSettings {
    pub fn default() -> Self {
        Self {
            selected_item: FrameEmissionSettingsItems::Bitrate,
        }
    }
}

impl DisplayManager<FrameEmissionSettings> {
    pub fn up_or_down_pressed(&mut self) {
        match self.current_screen.selected_item {
            FrameEmissionSettingsItems::Bitrate => {
                self.current_screen.selected_item = FrameEmissionSettingsItems::Mode
            }
            FrameEmissionSettingsItems::Mode => {
                self.current_screen.selected_item = FrameEmissionSettingsItems::Bitrate
            }
        }
    }

    pub fn right_pressed(&mut self) {
        match self.current_screen.selected_item {
            FrameEmissionSettingsItems::Bitrate => self.bitrate.increment(),
            FrameEmissionSettingsItems::Mode => self.mode.to_next(),
        }
    }

    pub fn left_pressed(&mut self) {
        match self.current_screen.selected_item {
            FrameEmissionSettingsItems::Bitrate => self.bitrate.decrement(),
            FrameEmissionSettingsItems::Mode => self.mode.to_next(),
        }
    }

    pub fn ok_pressed(&mut self) {
        todo!("GOTO FrameEmission")
    }
}

// ############################
// ### CAPTUREFILESELECTION ###
// ############################
#[derive(Debug)]
struct CaptureFileSelection {}
impl DisplayManager<CaptureFileSelection> {
    // procedures relatives à l'écran CaptureFileSelection
}
impl DisplayScreen for CaptureFileSelection {}

// ####################
// ### FRAMECAPTURE ###
// ####################
#[derive(Debug)]
struct FrameCapture {
    is_silent: bool,
}
impl DisplayScreen for FrameCapture {}

impl FrameCapture {
    pub fn default() -> Self {
        Self { is_silent: false }
    }
}

impl DisplayManager<FrameCapture> {
    // procedures relatives à l'écran FrameCapture
    pub fn up_pressed(&mut self) {
        self.bitrate.increment()
    }

    pub fn down_pressed(&mut self) {
        self.bitrate.decrement()
    }

    pub fn right_pressed(&mut self) {
        self.current_screen.is_silent = !self.current_screen.is_silent
    }

    pub fn left_pressed(&mut self) {
        todo!("GOTO CaptureFileSelection OR Home")
    }
}
