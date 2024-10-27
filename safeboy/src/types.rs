use std::any::Any;
use std::ffi::{c_char, CStr};
use std::mem::transmute;
use sameboy_sys::*;
use crate::Gameboy;

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum Model {
    DMGB = GB_model_t_GB_MODEL_DMG_B as u32,

    SGBNTSC = GB_model_t_GB_MODEL_SGB_NTSC as u32,
    SGBPAL = GB_model_t_GB_MODEL_SGB_PAL as u32,
    SGBNTSCNoSFC = GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC as u32,
    SGBPALNoSFC = GB_model_t_GB_MODEL_SGB_PAL_NO_SFC as u32,
    SGB2 = GB_model_t_GB_MODEL_SGB2 as u32,
    SGB2NoSFC = GB_model_t_GB_MODEL_SGB2_NO_SFC as u32,

    MGB = GB_model_t_GB_MODEL_MGB as u32,

    CGB0 = GB_model_t_GB_MODEL_CGB_0 as u32,
    CGBA = GB_model_t_GB_MODEL_CGB_A as u32,
    CGBB = GB_model_t_GB_MODEL_CGB_B as u32,
    CGBC = GB_model_t_GB_MODEL_CGB_C as u32,
    CGBD = GB_model_t_GB_MODEL_CGB_D as u32,
    CGBE = GB_model_t_GB_MODEL_CGB_E as u32,

    AGBA = GB_model_t_GB_MODEL_AGB_A as u32,
    GBPA = GB_model_t_GB_MODEL_GBP_A as u32,
}

impl TryFrom<u32> for Model {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            sameboy_sys::GB_model_t_GB_MODEL_DMG_B => { Ok(Self::DMGB) },
            sameboy_sys::GB_model_t_GB_MODEL_SGB_NTSC => { Ok(Self::SGBNTSC) },
            sameboy_sys::GB_model_t_GB_MODEL_SGB_PAL => { Ok(Self::SGBPAL) },
            sameboy_sys::GB_model_t_GB_MODEL_SGB_NTSC_NO_SFC => { Ok(Self::SGBNTSCNoSFC) },
            sameboy_sys::GB_model_t_GB_MODEL_SGB_PAL_NO_SFC => { Ok(Self::SGBPALNoSFC) },
            sameboy_sys::GB_model_t_GB_MODEL_SGB2 => { Ok(Self::SGB2) },
            sameboy_sys::GB_model_t_GB_MODEL_SGB2_NO_SFC => { Ok(Self::SGB2NoSFC) },
            sameboy_sys::GB_model_t_GB_MODEL_MGB => { Ok(Self::MGB) },
            sameboy_sys::GB_model_t_GB_MODEL_CGB_0 => { Ok(Self::CGB0) },
            sameboy_sys::GB_model_t_GB_MODEL_CGB_A => { Ok(Self::CGBA) },
            sameboy_sys::GB_model_t_GB_MODEL_CGB_B => { Ok(Self::CGBB) },
            sameboy_sys::GB_model_t_GB_MODEL_CGB_C => { Ok(Self::CGBC) },
            sameboy_sys::GB_model_t_GB_MODEL_CGB_D => { Ok(Self::CGBD) },
            sameboy_sys::GB_model_t_GB_MODEL_CGB_E => { Ok(Self::CGBE) },
            sameboy_sys::GB_model_t_GB_MODEL_AGB_A => { Ok(Self::AGBA) },
            sameboy_sys::GB_model_t_GB_MODEL_GBP_A => { Ok(Self::GBPA) },
            _ => Err(())
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum AudioChannel {
    Square1 = GB_channel_t_GB_SQUARE_1 as u32,
    Square2 = GB_channel_t_GB_SQUARE_2 as u32,
    Wave = GB_channel_t_GB_WAVE as u32,
    Noise = GB_channel_t_GB_NOISE as u32,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum ColorCorrectionMode {
    Disabled = GB_color_correction_mode_t_GB_COLOR_CORRECTION_DISABLED as u32,
    CorrectCurves = GB_color_correction_mode_t_GB_COLOR_CORRECTION_CORRECT_CURVES as u32,
    ModernBalanced = GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BALANCED as u32,
    BoostContrast = GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BOOST_CONTRAST as u32,
    ReduceContrast = GB_color_correction_mode_t_GB_COLOR_CORRECTION_REDUCE_CONTRAST as u32,
    LowContrast = GB_color_correction_mode_t_GB_COLOR_CORRECTION_LOW_CONTRAST as u32,
    ModernAccurate = GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_ACCURATE as u32,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum HighpassFilterMode {
    /// Do not apply any filter, keep DC offset
    Off = GB_highpass_mode_t_GB_HIGHPASS_OFF as u32,

    /// Apply a highpass filter similar to the one used on hardware
    Accurate = GB_highpass_mode_t_GB_HIGHPASS_ACCURATE as u32,

    /// Remove DC Offset without affecting the waveform
    RemoveDCOffset = GB_highpass_mode_t_GB_HIGHPASS_REMOVE_DC_OFFSET as u32,

    Max = GB_highpass_mode_t_GB_HIGHPASS_MAX as u32
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum SGBBorderMode {
    SGB = GB_border_mode_t_GB_BORDER_SGB as u32,
    Never = GB_border_mode_t_GB_BORDER_NEVER as u32,
    Always = GB_border_mode_t_GB_BORDER_ALWAYS as u32,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum Key {
    Right = GB_key_t_GB_KEY_RIGHT as u8,
    Left = GB_key_t_GB_KEY_LEFT as u8,
    Up = GB_key_t_GB_KEY_UP as u8,
    Down = GB_key_t_GB_KEY_DOWN as u8,
    A = GB_key_t_GB_KEY_A as u8,
    B = GB_key_t_GB_KEY_B as u8,
    Select = GB_key_t_GB_KEY_SELECT as u8,
    Start = GB_key_t_GB_KEY_START as u8,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum KeyMaskBit {
    Right = GB_key_mask_t_GB_KEY_RIGHT_MASK as u8,
    Left = GB_key_mask_t_GB_KEY_LEFT_MASK as u8,
    Up = GB_key_mask_t_GB_KEY_UP_MASK as u8,
    Down = GB_key_mask_t_GB_KEY_DOWN_MASK as u8,
    A = GB_key_mask_t_GB_KEY_A_MASK as u8,
    B = GB_key_mask_t_GB_KEY_B_MASK as u8,
    Select = GB_key_mask_t_GB_KEY_SELECT_MASK as u8,
    Start = GB_key_mask_t_GB_KEY_START_MASK as u8,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum Rumble {
    Disabled = GB_rumble_mode_t_GB_RUMBLE_DISABLED as u32,
    CartridgeOnly = GB_rumble_mode_t_GB_RUMBLE_CARTRIDGE_ONLY as u32,
    AllGames = GB_rumble_mode_t_GB_RUMBLE_ALL_GAMES as u32
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum RTCMode {
    SyncToHost = GB_rtc_mode_t_GB_RTC_MODE_SYNC_TO_HOST as u32,
    Accurate = GB_rtc_mode_t_GB_RTC_MODE_ACCURATE as u32,
}

#[derive(Copy, Clone)]
pub struct Registers {
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub pc: u16
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum Accessory {
    None = GB_accessory_t_GB_ACCESSORY_NONE as u32,
    Printer = GB_accessory_t_GB_ACCESSORY_PRINTER as u32,
    Workboy = GB_accessory_t_GB_ACCESSORY_WORKBOY as u32,
}
impl From<GB_accessory_t> for Accessory {
    fn from(value: GB_accessory_t) -> Self {
        match value {
            sameboy_sys::GB_accessory_t_GB_ACCESSORY_WORKBOY => Accessory::Workboy,
            sameboy_sys::GB_accessory_t_GB_ACCESSORY_PRINTER => Accessory::Printer,
            sameboy_sys::GB_accessory_t_GB_ACCESSORY_NONE => Accessory::None,
            _ => unreachable!()
        }
    }
}

impl From<GB_registers_t> for Registers {
    fn from(value: GB_registers_t) -> Self {
        let registers = unsafe { value.__bindgen_anon_1 };
        Self {
            af: registers.af,
            bc: registers.bc,
            de: registers.de,
            hl: registers.hl,
            sp: registers.sp,
            pc: registers.pc
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum DirectAccess {
    ROM = GB_direct_access_t_GB_DIRECT_ACCESS_ROM as u32,
    RAM = GB_direct_access_t_GB_DIRECT_ACCESS_RAM as u32,
    CARTRAM = GB_direct_access_t_GB_DIRECT_ACCESS_CART_RAM as u32,
    VRAM = GB_direct_access_t_GB_DIRECT_ACCESS_VRAM as u32,
    HRAM = GB_direct_access_t_GB_DIRECT_ACCESS_HRAM as u32,

    /// Warning: Some registers can only be read/written correctly via GB_memory_read/write.
    IO = GB_direct_access_t_GB_DIRECT_ACCESS_IO as u32,
    BOOTROM = GB_direct_access_t_GB_DIRECT_ACCESS_BOOTROM as u32,
    OAM = GB_direct_access_t_GB_DIRECT_ACCESS_OAM as u32,
    BGP = GB_direct_access_t_GB_DIRECT_ACCESS_BGP as u32,
    OBP = GB_direct_access_t_GB_DIRECT_ACCESS_OBP as u32,
    IE = GB_direct_access_t_GB_DIRECT_ACCESS_IE as u32,

    /// Identical to ROM, but returns the correct rom0 bank in the bank output argument
    ROM0 = GB_direct_access_t_GB_DIRECT_ACCESS_ROM0 as u32,
}

#[derive(Clone)]
pub struct PrinterPage {
    pub data: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub top_margin: usize,
    pub bottom_margin: usize,
    pub exposure: u8
}

#[derive(Copy, Clone, PartialEq)]
#[repr(u32)]
pub enum VBlankType {
    /// An actual Vblank-triggered frame
    NormalFrame = GB_vblank_type_t_GB_VBLANK_TYPE_NORMAL_FRAME,

    /// An artificial frame pushed while the LCD was off
    LCDOff = GB_vblank_type_t_GB_VBLANK_TYPE_LCD_OFF,

    /// An artificial frame pushed for some other reason
    Artificial = GB_vblank_type_t_GB_VBLANK_TYPE_ARTIFICIAL,

    /// A frame that would not render on actual hardware, but the screen should retain the previous frame
    Repeat = GB_vblank_type_t_GB_VBLANK_TYPE_REPEAT,
}

impl From<GB_vblank_type_t> for VBlankType {
    fn from(value: GB_vblank_type_t) -> Self {
        match value {
            sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_NORMAL_FRAME => VBlankType::NormalFrame,
            sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_LCD_OFF => VBlankType::LCDOff,
            sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_ARTIFICIAL => VBlankType::Artificial,
            sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_REPEAT => VBlankType::Repeat,
            _ => unreachable!()
        }
    }
}

pub type Palette = [PaletteColor; 5];

#[derive(Copy, Clone)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl From<GB_palette_t_GB_color_s> for PaletteColor {
    fn from(value: GB_palette_t_GB_color_s) -> Self {
        let GB_palette_t_GB_color_s { r, g, b } = value;
        Self { r, g, b }
    }
}

impl From<PaletteColor> for GB_palette_t_GB_color_s {
    fn from(value: PaletteColor) -> Self {
        let PaletteColor { r, g, b } = value;
        Self { r, g, b }
    }
}

#[derive(Clone)]
pub struct GBSInfo {
    pub track_count: u8,
    pub first_track: u8,
    pub title: String,
    pub author: String,
    pub copyright: String,
}

impl From<GB_gbs_info_t> for GBSInfo {
    fn from(value: GB_gbs_info_t) -> Self {
        Self {
            track_count: 0,
            first_track: 0,
            title: CStr::from_bytes_until_nul(&unsafe { transmute::<[c_char; 33], [u8; 33]>(value.title) })
                .expect("bad title")
                .to_string_lossy()
                .to_string(),
            author: CStr::from_bytes_until_nul(&unsafe { transmute::<[c_char; 33], [u8; 33]>(value.author) })
                .expect("bad author")
                .to_string_lossy()
                .to_string(),
            copyright: CStr::from_bytes_until_nul(&unsafe { transmute::<[c_char; 33], [u8; 33]>(value.copyright) })
                .expect("bad copyright")
                .to_string_lossy()
                .to_string(),
        }
    }
}

#[derive(Copy, Clone)]
pub enum RgbEncoding {
    B8G8R8X8,
    R8G8B8X8,
    X8R8G8B8,
    X8B8G8R8,
}

#[derive(Default, Copy, Clone)]
pub struct EnabledEvents {
    /// Record vblank events.
    ///
    /// Note that vblank events will still be recorded even if rendering is disabled.
    pub vblank: bool,

    /// Record audio samples.
    ///
    /// A sample rate must be enabled with [`Gameboy::set_sample_rate`] or [`Gameboy::set_sample_rate_by_clocks`].
    pub sample: bool,

    /// Record rumble events.
    ///
    /// Rumble must be enabled with [`Gameboy::set_rumble_mode`].
    pub rumble: bool,

    /// Record printer events.
    ///
    /// An emulated printer must be connected with [`Gameboy::connect_printer`].
    pub printer: bool,

    /// Record all memory reads.
    pub memory_read: bool,

    /// Record all memory writes.
    pub memory_write: bool,
}

pub type WriteMemoryCallback = fn(user_data: Option<&mut dyn Any>, address: u16, data: u8) -> bool;
pub type ReadMemoryCallback = fn(user_data: Option<&mut dyn Any>, address: u16, data: u8) -> u8;
