#![expect(unsafe_op_in_unsafe_fn)]

use crate::{BootRomType, GameboyCallbacks, RunningGameboy, LogAttributes, PrinterPage, VBlankType};
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};
use core::ptr::null_mut;
use sameboy_sys::{GB_boot_rom_t, GB_gameboy_s, GB_gameboy_t, GB_get_user_data, GB_log_attributes_t, GB_log_attributes_t_GB_LOG_BOLD, GB_log_attributes_t_GB_LOG_DASHED_UNDERLINE, GB_sample_t, GB_vblank_type_t};

// SAFETY: The user data should have been set up already.
//
// Note: Technically this is already mutably borrowed. However, we're currently inside of a function
// call from that mutable borrow. So there isn't really any concern about thread safety here, though
// we should still be really careful that this reference doesn't leak (static mutable borrows are
// scary as it is).
unsafe fn get_instance(gb: *mut GB_gameboy_s) -> &'static mut RunningGameboy {
    &mut *(GB_get_user_data(gb) as *mut _ as *mut RunningGameboy)
}

unsafe fn get_instance_callback(gb: *mut GB_gameboy_s) -> &'static mut dyn GameboyCallbacks {
    get_instance(gb).callbacks.as_mut()
}

pub(crate) unsafe extern "C" fn rgb_encode_callback(gb: *mut GB_gameboy_s, r: u8, g: u8, b: u8) -> u32 {
    (get_instance(gb).rgb_encoder)(r, g, b)
}

pub unsafe extern "C" fn read_memory_callback(gb: *mut GB_gameboy_t, address: u16, original_data: u8) -> u8 {
    get_instance_callback(gb).read_memory(get_instance(gb), address, original_data)
}

pub unsafe extern "C" fn write_memory_callback(gb: *mut GB_gameboy_t, address: u16, data: u8) -> bool {
    get_instance_callback(gb).write_memory(get_instance(gb), address, data)
}

pub unsafe extern "C" fn execution_callback(gb: *mut GB_gameboy_t, pc: u16, opcode: u8) {
    get_instance_callback(gb).executing_instruction(get_instance(gb), pc, opcode);
}

pub unsafe extern "C" fn serial_transfer_bit_start_callback(gb: *mut GB_gameboy_t, value: bool) {
    get_instance_callback(gb).serial_transfer_bit_start(get_instance(gb), value);
}

pub unsafe extern "C" fn serial_transfer_bit_end_callback(gb: *mut GB_gameboy_t) -> bool {
    get_instance_callback(gb).serial_transfer_bit_end(get_instance(gb))
}

pub unsafe extern "C" fn vblank_callback(gb: *mut GB_gameboy_t, vblank_type: GB_vblank_type_t) {
    let vblank_type = match vblank_type {
        sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_NORMAL_FRAME => VBlankType::Normal,
        sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_SKIPPED_FRAME => VBlankType::SkippedFrame,
        sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_ARTIFICIAL => VBlankType::Artificial,
        sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_LCD_OFF => VBlankType::LCDOff,
        sameboy_sys::GB_vblank_type_t_GB_VBLANK_TYPE_REPEAT => VBlankType::Repeat,
        unknown => panic!("Unknown vblank type {unknown}")
    };
    get_instance_callback(gb).vblank(get_instance(gb), vblank_type);
}

pub unsafe extern "C" fn update_input_hint_callback(gb: *mut GB_gameboy_t) {
    get_instance_callback(gb).update_input_hint(get_instance(gb));
}

pub unsafe extern "C" fn load_boot_rom_callback(gb: *mut GB_gameboy_t, boot_rom_type: GB_boot_rom_t) {
    let boot_rom_type = match boot_rom_type {
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_DMG_0 => BootRomType::Dmg0,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_DMG => BootRomType::Dmg,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_MGB => BootRomType::Mgb,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_SGB => BootRomType::Sgb,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_SGB2 => BootRomType::Sgb2,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_CGB_0 => BootRomType::Cgb0,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_CGB => BootRomType::Cgb,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_CGB_E => BootRomType::CgbE,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_AGB_0 => BootRomType::Agb0,
        sameboy_sys::GB_boot_rom_t_GB_BOOT_ROM_AGB => BootRomType::Agb,
        unknown => panic!("Unknown boot rom type {unknown}")
    };

    get_instance_callback(gb).load_boot_rom_hint(get_instance(gb), boot_rom_type);
}

pub unsafe extern "C" fn infrared_callback(gb: *mut GB_gameboy_t, on: bool) {
    get_instance_callback(gb).infrared(get_instance(gb), on);
}

pub unsafe extern "C" fn rumble_callback(gb: *mut GB_gameboy_t, amplitude: f64) {
    get_instance_callback(gb).rumble(get_instance(gb), amplitude);
}

pub unsafe extern "C" fn apu_sample_callback(gb: *mut GB_gameboy_t, samples: *mut GB_sample_t) {
    let samples = *samples;
    get_instance_callback(gb).apu_sample(get_instance(gb), samples.left, samples.right);
}

pub unsafe extern "C" fn log_callback(gb: *mut GB_gameboy_t, string: *const c_char, attributes: GB_log_attributes_t) {
    let string = CStr::from_ptr(string).to_string_lossy();

    let parameters = LogAttributes {
        bold: (attributes & GB_log_attributes_t_GB_LOG_BOLD) != 0,
        dashed_underline: (attributes & GB_log_attributes_t_GB_LOG_DASHED_UNDERLINE) != 0,
        underline: (attributes & GB_log_attributes_t_GB_LOG_DASHED_UNDERLINE) != 0,
    };

    get_instance_callback(gb).console_log(get_instance(gb), string.as_ref(), parameters);
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
}

pub unsafe extern "C" fn input_callback(gb: *mut GB_gameboy_t) -> *mut c_char {
    let Some(input) = get_instance_callback(gb).console_input(get_instance(gb)) else {
        return null_mut()
    };
    let input_bytes = input.as_ref().as_bytes();

    assert_eq!(size_of::<c_char>(), size_of::<u8>());

    let len = input_bytes.len() + 1;
    let data = malloc(len);
    if data.is_null() {
        panic!("failed to allocate buffer for input from input_callback");
    }

    let data_slice = core::slice::from_raw_parts_mut(data, len);
    let (before, after) = data_slice.split_at_mut(len);
    before.copy_from_slice(input_bytes);
    after.fill(0);

    data as *mut c_char
}

pub(crate) unsafe extern "C" fn printer_callback(
    gb: *mut GB_gameboy_t,
    image: *mut u32,
    height: u8,
    top_margin: u8,
    bottom_margin: u8,
    exposure: u8
) {
    let width = 160;
    let mut page = PrinterPage {
        width,
        top_margin,
        data: Vec::new(),
        content_height: height,
        bottom_margin,
        exposure
    };

    let width = width as usize;
    let height = height as usize;
    let top_margin = top_margin as usize;
    let bottom_margin = bottom_margin as usize;
    let total_height = height + top_margin + bottom_margin;

    let mut data = vec![0xFFFFFFFF; total_height * width];
    let input_data = unsafe { core::slice::from_raw_parts_mut(image, height * width) };
    let output_start = width * top_margin;
    data[output_start..output_start + height * width].copy_from_slice(input_data);

    page.data = data;

    get_instance_callback(gb).printer_page(get_instance(gb), page);
}

pub(crate) extern "C" fn printer_done_callback(_: *mut GB_gameboy_t) {}
