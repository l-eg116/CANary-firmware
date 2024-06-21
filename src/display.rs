use crate::can::Bitrate;

enum EmissionMode {
    WaitACK,
    NoACK,
    Loopback,
}

struct DisplayManager<STATE> {
    pub current_screen: STATE,
    pub bitrate: Bitrate,
    pub mode: EmissionMode,
}

impl<STATE> DisplayManager<STATE> {
    // procedures communes à tous les écrans
    pub fn default() -> DisplayManager<Home> {
        DisplayManager {
            current_screen: Home::default(),
            bitrate: Bitrate::Br125kbps,
            mode: EmissionMode::WaitACK,
        }
    }
}

// ############
// ### HOME ###
// ############
enum HomeItems {
    None,
    Emit,
    Capture,
} //noms non contractuels
struct Home {
    pub selected_item: HomeItems,
}

impl Home {
    pub fn default() -> Home {
        Home {
            selected_item: HomeItems::None,
        }
    }
}

impl DisplayManager<Home> {
    pub fn right_or_left_pressed(&mut self) {
        match self.current_screen.selected_item {
            // first state is None, then alternate between Capture and Emit
            HomeItems::None => self.current_screen.selected_item = HomeItems::Capture,
            HomeItems::Emit => self.current_screen.selected_item = HomeItems::Capture,
            HomeItems::Capture => self.current_screen.selected_item = HomeItems::Emit,
        }
    }

    pub fn ok_pressed(&mut self) {
        match self.current_screen.selected_item {
            HomeItems::Emit => todo!("GOTO EmissionFrameSelection"),
            HomeItems::Capture => todo!("GOTO CaptureFileSelection"),
            _ => (),
        }
    }
}

// ##############################
// ### EMISSIONFRAMESELECTION ###
// ##############################
// enum EmissionFrameSelectionItems {None, DisplayMode}
struct EmissionFrameSelection {
    // pub selected_item: EmissionFrameSelectionItems
}

impl DisplayManager<EmissionFrameSelection> {
    // procedures relatives à l'écran EmissionFrameSelection
}

// #####################
// ### FRAMEEMISSION ###
// #####################
struct FrameEmission {
    pub frame_count: u8,
    pub is_sending_frames: bool,
}

impl FrameEmission {
    pub fn default() -> FrameEmission {
        FrameEmission {
            frame_count: 1,
            is_sending_frames: false,
        }
    }
}

impl DisplayManager<FrameEmission> {
    pub fn up_pressed(&mut self) {
        self.current_screen.frame_count += 1
    }

    pub fn down_pressed(&mut self) {
        if self.current_screen.frame_count != 0 {
            self.current_screen.frame_count -= 1
        }
    }

    pub fn right_pressed(&mut self) {
        todo!("GOTO FrameEmissionSettings")
    }

    pub fn left_pressed(&mut self) {
        todo!("GOTO EmissionFrameSelection OR Home ?")
    }

    pub fn ok_pressed(&mut self) {
        self.current_screen.is_sending_frames = !self.current_screen.is_sending_frames
    }
}

// #############################
// ### FRAMEEMISSIONSETTINGS ###
// #############################
enum FrameEmissionSettingsItems {
    Bitrate,
    Mode,
    Ok,
}

struct FrameEmissionSettings {
    pub selected_item: FrameEmissionSettingsItems,
}

impl FrameEmissionSettings {
    pub fn default() -> FrameEmissionSettings {
        FrameEmissionSettings {
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
                self.current_screen.selected_item = FrameEmissionSettingsItems::Ok
            }
            FrameEmissionSettingsItems::Ok => {
                self.current_screen.selected_item = FrameEmissionSettingsItems::Bitrate
            }
        }
    }

    pub fn right_pressed(&mut self) {
        match self.current_screen.selected_item {
            FrameEmissionSettingsItems::Bitrate => self.bitrate.increment(),
            FrameEmissionSettingsItems::Mode => match self.mode {
                EmissionMode::WaitACK => self.mode = EmissionMode::NoACK,
                EmissionMode::NoACK => self.mode = EmissionMode::Loopback,
                EmissionMode::Loopback => self.mode = EmissionMode::WaitACK,
            },
            _ => (),
        }
    }

    pub fn left_pressed(&mut self) {
        match self.current_screen.selected_item {
            FrameEmissionSettingsItems::Bitrate => self.bitrate.decrement(),
            FrameEmissionSettingsItems::Mode => match self.mode {
                EmissionMode::WaitACK => self.mode = EmissionMode::Loopback,
                EmissionMode::NoACK => self.mode = EmissionMode::WaitACK,
                EmissionMode::Loopback => self.mode = EmissionMode::NoACK,
            },
            _ => (),
        }
    }

    pub fn ok_pressed(&mut self) {
        match self.current_screen.selected_item {
            FrameEmissionSettingsItems::Ok => todo!("GOTO FrameEmission"),
            _ => (),
        }
    }
}

// ############################
// ### CAPTUREFILESELECTION ###
// ############################
struct CaptureFileSelection {}
impl DisplayManager<CaptureFileSelection> {
    // procedures relatives à l'écran CaptureFileSelection
}

// ####################
// ### FRAMECAPTURE ###
// ####################

struct FrameCapture {
    is_silent: bool,
}

impl FrameCapture {
    pub fn default() -> FrameCapture {
        FrameCapture { is_silent: false }
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
