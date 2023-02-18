use nom_derive::NomLE;

use crate::{Bool32, Data32, String32, ZlibImage};

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct GameSettings {
    #[nom(Verify = "*ver == 702")]
    pub ver: u32,
    pub fullscreen: Bool32,
    pub interpolate: Bool32,
    pub borderless: Bool32,
    pub show_cursor: Bool32,
    pub scaling: i32,
    pub resizable: Bool32,
    pub always_on_top: Bool32,
    pub background_color: u32,
    pub set_resolution: Bool32,
    pub color_depth: ColorDepth,
    pub resolution: Resolution,
    pub frequency: Frequency,
    pub dont_show_buttons: Bool32,
    pub sync: Bool32, // actually, software vertex processing in high bit.
    // if ver >= 800 disable_screensavers: Bool32,
    pub let_f4_fullscreen: Bool32,
    pub let_f1_help: Bool32,
    pub let_escape_end_game: Bool32,
    pub let_f5_save_f6_load: Bool32,
    pub let_f9_screenshot: Bool32,
    pub close_as_escape: Bool32,
    pub priority: Priority,
    pub freeze_in_background: Bool32,
    pub progress_bar: ProgressBar,
    #[nom(Cond = "progress_bar == ProgressBar::Custom")]
    pub progress_bar_custom_back: Option<ZlibImage>,
    #[nom(Cond = "progress_bar == ProgressBar::Custom")]
    pub progress_bar_custom_front: Option<ZlibImage>,
    pub show_custom_load_image: Bool32,
    pub custom_load_image: ZlibImage,
    pub image_partially_transparent: Bool32,
    pub image_alpha: u32,
    pub scale_progress_bar: Bool32,
    pub icon: Data32,
    pub display_errors: Bool32,
    pub write_to_log: Bool32,
    pub abort_on_error: Bool32,
    pub error_flags: u32,
    pub author: String32,
    pub version: String32,
    pub last_changed: f64,
    pub information: String32,
    #[nom(LengthCount = "nom::number::complete::le_u32")]
    pub constants: Vec<Constant>,
    pub version_major: u32,
    pub version_minor: u32,
    pub version_release: u32,
    pub version_build: u32,
    pub company: String32,
    pub product: String32,
    pub copyright: String32,
    pub description: String32,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ColorDepth {
    NoChange = 0,
    _16 = 1,
    _32 = 2,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum Resolution {
    NoChange = 0,
    _640x480 = 1,
    _800x600 = 2,
    _1024x768 = 3,
    _1280x1024 = 4,
    _1600x1200 = 5,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum Frequency {
    NoChange = 0,
    _60 = 1,
    _70 = 2,
    _85 = 3,
    _100 = 4,
    _120 = 5,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum Priority {
    Normal,
    High,
    Highest,
}

#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, Debug, NomLE)]
#[nom(GenericErrors)]
pub enum ProgressBar {
    None,
    Default,
    Custom,
}

#[derive(Debug, NomLE)]
#[nom(GenericErrors)]
pub struct Constant {
    pub name: String32,
    pub value: String32,
}
