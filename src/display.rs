use crate::can::Bitrate;

enum EmissionMode {
    WaitACK,
    NoACK,
    Loopback,
}

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
            bitrate: Bitrate::Br125kbps,
            mode: EmissionMode::WaitACK,
        }
    }
}

trait DisplayScreen {}

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
impl DisplayScreen for Home {}

impl Home {
    pub fn default() -> Self {
        Self {
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
impl DisplayScreen for EmissionFrameSelection {}

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
impl DisplayScreen for FrameEmissionSettings {}

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
impl DisplayScreen for CaptureFileSelection {}

// ####################
// ### FRAMECAPTURE ###
// ####################

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
