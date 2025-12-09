use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void, CStr};
use core::marker::PhantomPinned;
use core::mem::transmute;
use core::ops::{Shl, ShlAssign};
use sameboy_sys::{GB_alloc, GB_apu_set_sample_callback, GB_connect_printer, GB_dealloc, GB_gameboy_t, GB_get_clock_rate, GB_get_direct_access, GB_get_palette, GB_get_registers, GB_get_rom_title, GB_get_sample_rate, GB_get_save_state_size, GB_get_screen_height, GB_get_screen_width, GB_get_unmultiplied_clock_rate, GB_get_usual_frame_rate, GB_init, GB_is_background_rendering_disabled, GB_is_cgb, GB_is_cgb_in_cgb_mode, GB_is_hle_sgb, GB_is_object_rendering_disabled, GB_is_odd_frame, GB_is_sgb, GB_load_battery_from_buffer, GB_load_boot_rom_from_buffer, GB_load_rom_from_buffer, GB_load_state_from_buffer, GB_model_t, GB_palette_t, GB_palette_t_GB_color_s, GB_quick_reset, GB_reset, GB_rewind_pop, GB_rewind_reset, GB_run, GB_run_frame, GB_save_battery_size, GB_save_battery_to_buffer, GB_save_state_to_buffer, GB_set_allow_illegal_inputs, GB_set_background_rendering_disabled, GB_set_boot_rom_load_callback, GB_set_border_mode, GB_set_clock_multiplier, GB_set_color_correction_mode, GB_set_execution_callback, GB_set_infrared_callback, GB_set_input_callback, GB_set_key_mask, GB_set_key_state, GB_set_light_temperature, GB_set_log_callback, GB_set_object_rendering_disabled, GB_set_palette, GB_set_pixels_output, GB_set_read_memory_callback, GB_set_rendering_disabled, GB_set_rewind_length, GB_set_rgb_encode_callback, GB_set_rtc_mode, GB_set_rumble_callback, GB_set_sample_rate, GB_set_serial_transfer_bit_end_callback, GB_set_serial_transfer_bit_start_callback, GB_set_turbo_mode, GB_set_update_input_hint_callback, GB_set_user_data, GB_set_vblank_callback, GB_set_write_memory_callback, GB_switch_model_and_reset};

pub(crate) mod callback_wrapper;
mod callbacks;
use crate::instance::callback_wrapper::*;
use crate::instance::NullCallbacks;
use crate::rgb_encoder::{encode_a8r8g8b8, RgbEncoder};
use crate::{Gameboy, Model};
pub use callbacks::*;

/// Describes a running instance.
///
/// Some functions are not available on a running instance such as loading ROMs.
pub struct RunningGameboy {
    gb: *mut GB_gameboy_t,
    pub(crate) callbacks: Box<dyn GameboyCallbacks>,
    pixel_buffer: Vec<u32>,
    screen_width: u16,
    screen_height: u16,
    pub(crate) is_running: bool,
    pub(crate) rgb_encoder: RgbEncoder,
    rom_title: String,
    _unpin: PhantomPinned
}

impl RunningGameboy {
    pub(crate) fn new(model: Model) -> Self {
        // SAFETY: This is safe and is provided as an example
        let sameboy = unsafe { GB_init(GB_alloc(), model as GB_model_t) };

        let mut instance = RunningGameboy {
            gb: sameboy,
            callbacks: Box::new(NullCallbacks),
            pixel_buffer: Vec::new(),
            screen_width: 0,
            screen_height: 0,
            rgb_encoder: encode_a8r8g8b8,
            is_running: false,
            rom_title: String::new(),
            _unpin: PhantomPinned
        };
        instance.reset_pixel_buffer();
        instance
    }

    pub(crate) fn reset_pixel_buffer(&mut self) {
        self.pixel_buffer.clear();
        self.screen_width = u16::try_from(unsafe { GB_get_screen_width(self.gb) }).expect("screen width does not fit into a u16");
        self.screen_height = u16::try_from(unsafe { GB_get_screen_height(self.gb) }).expect("screen height does not fit into a u16");
        self.pixel_buffer.resize(self.screen_width as usize * self.screen_height as usize, 0);
        unsafe { GB_set_pixels_output(self.gb, self.pixel_buffer.as_mut_ptr()) };
    }

    pub(crate) fn set_callbacks(&mut self, callbacks: Option<Box<dyn GameboyCallbacks>>) {
        self.assert_not_running();
        self.callbacks = callbacks.unwrap_or(Box::new(NullCallbacks));
        self.setup_callbacks();
    }

    fn assert_not_running(&self) {
        // This should never happen for these three reasons:
        //
        // - The run and run_frame functions require mutable borrows
        // - Rust's borrow checker prevents you from borrowing something already mutably borrowed
        // - All functions that cannot be called while running are not publicly accessible from
        //   RunningInstance
        //
        assert!(!self.is_running, "a function was run while the emulator was running (THIS SHOULD NEVER HAPPEN!)")
    }

    pub(super) fn run(&mut self) -> u32 {
        self.assert_not_running();
        unsafe {
            self.is_running = true;
            let return_value = GB_run(self.gb);
            self.is_running = false;
            return_value
        }
    }

    pub(super) fn run_frame(&mut self) -> u64 {
        self.assert_not_running();
        unsafe {
            self.is_running = true;
            let return_value = GB_run_frame(self.gb);
            self.is_running = false;
            return_value
        }
    }

    pub(super) fn reset(&mut self) {
        self.assert_not_running();
        unsafe { GB_reset(self.gb) };
    }

    pub(super) fn partial_reset(&mut self) {
        self.assert_not_running();
        unsafe { GB_quick_reset(self.gb) };
    }

    pub(super) fn rewind_pop(&mut self)-> bool {
        self.assert_not_running();
        unsafe { GB_rewind_pop(self.gb) }
    }

    pub(super) fn switch_model_and_reset(&mut self, model: Model) {
        self.assert_not_running();
        unsafe { GB_switch_model_and_reset(self.gb, model as GB_model_t) };
        self.reset_pixel_buffer();
    }

    pub(super) fn create_save_state(&self) -> Vec<u8> {
        self.assert_not_running();
        unsafe {
            let save_state_size = GB_get_save_state_size(self.gb);
            let mut data = vec![0u8; save_state_size];
            GB_save_state_to_buffer(self.gb, data.as_mut_ptr());
            data
        }
    }

    pub(super) fn load_save_state(&mut self, state: &[u8]) -> Result<(), crate::ReadSaveStateError> {
        self.assert_not_running();
        let error = unsafe {
            GB_load_state_from_buffer(self.gb, state.as_ptr(), state.len())
        };

        match error {
            0 => {
                self.reset_pixel_buffer();
                Ok(())
            },
            _ => Err(crate::ReadSaveStateError::Other)
        }
    }

    pub(super) fn finish_init(&mut self) {
        self.assert_not_running();

        // This MUST be set before we have any callbacks.
        unsafe { GB_set_user_data(self.gb, self as *mut _ as *mut c_void) };

        // RGB encoder
        self.set_rgb_encoder(encode_a8r8g8b8);
    }

    fn setup_callbacks(&mut self) {
        // Set callbacks and sane defaults so the callbacks work.
        unsafe {
            GB_set_vblank_callback(self.gb, Some(vblank_callback));
            GB_set_update_input_hint_callback(self.gb, Some(update_input_hint_callback));
            GB_set_boot_rom_load_callback(self.gb, Some(load_boot_rom_callback));
            GB_set_infrared_callback(self.gb, Some(infrared_callback));
            GB_set_rumble_callback(self.gb, Some(rumble_callback));
            GB_apu_set_sample_callback(self.gb, Some(apu_sample_callback));

            // Console stuff
            GB_set_log_callback(self.gb, Some(log_callback));
            GB_set_input_callback(self.gb, Some(input_callback));
            // won't do GB_set_async_input_callback yet since it's probably really unsafe

            // Camera stuff
            // TODO: GB_set_camera_get_pixel_callback
            // TODO: GB_set_camera_update_request_callback

            // SNES stuff
            // TODO: GB_set_lcd_line_callback(self.gb, Some());
            // TODO: GB_set_lcd_status_callback(self.gb, Some());
            // TODO: GB_set_icd_pixel_callback(self.gb, Some());
            // TODO: GB_set_icd_hreset_callback(self.gb, Some());
            // TODO: GB_set_icd_vreset_callback(self.gb, Some());
            // TODO: GB_set_joyp_write_callback(self.gb, Some());
        }
    }

    pub(super) fn fixup_rom_title(&mut self) {
        let mut name = [0u8; 17];
        unsafe { GB_get_rom_title(self.gb, name.as_mut_ptr() as *mut c_char) };
        let rom_title = CStr::from_bytes_until_nul(&name)
            .expect("fixup_rom_title without null terminator?")
            .to_string_lossy()
            .into_owned();
        self.rom_title = rom_title;
    }
}

impl Drop for RunningGameboy {
    fn drop(&mut self) {
        unsafe { GB_dealloc(self.gb) }
    }
}

/// Functions that are safe to run inside of callbacks.
pub trait RunnableInstanceFunctions {
    /// Return the dimensions of the pixel buffer in `(width, height)`.
    fn get_pixel_buffer_dimensions(&self) -> (u16, u16);

    /// Get the pixel buffer pixels.
    ///
    /// # Remarks
    ///
    /// Getting the pixel buffer mid-frame may result in tearing. It is recommended to wait until
    /// the vblank callback is reached.
    /// 
    /// Also, rendering should be enabled for this buffer to be updated.
    fn get_pixel_buffer_pixels(&self) -> &[u32];

    /// Get the pixel buffer pixels _and_ dimensions.
    ///
    /// # Remarks
    ///
    /// Getting the pixel buffer mid-frame may result in tearing. It is recommended to wait until
    /// the vblank callback is reached.
    /// 
    /// Also, rendering should be enabled for this buffer to be updated.
    fn get_pixel_buffer(&self) -> PixelBufferRead<'_>;

    /// Set the current state for one button.
    fn set_input_button_state(&mut self, button: InputButton, state: bool);

    /// Set the current state for all buttons.
    ///
    /// `state`, in this case, is a packed `u8` where each bit represents one of the eight buttons.
    ///
    /// For example `set_input_button_mask((1 << InputButton::A) | (1 << InputButton::B))` presses
    /// the A and B buttons while releasing all other buttons.
    fn set_input_button_mask(&mut self, state: u8);

    /// Load the boot ROM.
    fn load_boot_rom(&mut self, boot_rom: &[u8]);

    /// Connect a serial adapter.
    ///
    /// This activates the serial I/O callbacks.
    fn connect_serial(&mut self);

    /// Connect a printer.
    ///
    /// This activates the printer callbacks, but it will disable serial callbacks.
    ///
    /// Disconnect the printer with `disconnect_serial`.
    fn connect_printer(&mut self);

    /// Disconnect the serial adapter (and all serial and printer callbacks).
    fn disconnect_serial(&mut self);

    /// Set whether or not I/O and execution callbacks are enabled.
    ///
    /// By default, memory callbacks are **not** enabled.
    fn set_memory_callbacks_enabled(&mut self, enabled: bool);

    /// Set whether or not rendering is enabled.
    /// 
    /// If rendering is disabled, the pixel buffer won't be updated. Note, however, that the vblank
    /// callback will still fire regardless of if rendering is enabled.
    ///
    /// By default, rendering is **not** enabled.
    fn set_rendering_enabled(&mut self, enabled: bool);

    /// Set the clock speed multiplier of the emulator.
    ///
    /// For example `2.0` will emulate at double speed.
    fn set_clock_multiplier(&mut self, multiplier: f64);

    /// Get the unmodified clock rate.
    ///
    /// The clock multiplier is ignored in this.
    fn get_unmultiplied_clock_rate(&self) -> u32;

    /// Get the unmodified clock rate.
    ///
    /// The clock multiplier is respected in this.
    fn get_clock_rate(&self) -> u32;

    /// Load the ROM.
    fn load_rom(&mut self, rom: &[u8]);

    /// Get direct access to a given region.
    fn direct_access(&'_ self, access: DirectAccessRegion) -> DirectAccessData<'_>;

    /// Get direct access to a given region.
    fn direct_access_mut(&'_ mut self, access: DirectAccessRegion) -> DirectAccessDataMut<'_>;

    /// Returns true if the current emulator is a Game Boy Color.
    fn is_cgb(&self) -> bool;

    /// Returns true if the current emulator is a Game Boy Color in Game Boy Color mode.
    ///
    /// If false but `is_cgb() == true`, it's a Game Boy Color in backwards compatibility (i.e. DMG)
    /// mode.
    fn is_cgb_in_cgb_mode(&self) -> bool;

    /// Returns true if the current emulator is a Super Game Boy with HLE SNES emulation.
    fn is_hle_sgb(&self) -> bool;

    /// Returns true if the current emulator is a Super Game Boy.
    fn is_sgb(&self) -> bool;

    // TODO
    // fn set_open_bus_decay_time(&self, decay: u32);

    /// Set the SGB border mode.
    /// 
    /// The default value is [`BorderMode::SgbOnly`]
    fn set_border_mode(&mut self, mode: BorderMode);

    /// Set color correction mode.
    fn set_color_correction_mode(&mut self, mode: ColorCorrectionMode);

    /// Set the ambient light temperature.
    /// 
    /// This simulates an external light source being applied to a non front/backlit screen.
    ///
    /// The minimum value is -1.0 (1000K or "warm red"), and the maximum is 1.0 (12000K or "cool
    /// blue").
    ///
    /// The default is `0.0` (6500K).
    fn set_light_temperature(&mut self, temperature: f64);

    /// Set the palette for monochrome models.
    fn set_palette(&mut self, palette: MonochromePalette);

    /// Get the palette for monochrome models.
    fn get_palette(&self) -> MonochromePalette;

    // TODO
    // fn convert_rgb15

    // TODO
    // fn draw_tilemap
    // fn draw_tileset

    /// Get the expected frame rate of the emulator.
    fn get_usual_frame_rate(&self) -> f64;

    /// Returns true if the number of frames rendered is odd.
    ///
    /// This can be used for blending between two frames and choosing which frame buffer to use.
    fn is_odd_frame(&self) -> bool;

    /// Set whether or not background rendering is disabled.
    fn set_background_rendering_enabled(&mut self, enabled: bool);

    /// Get whether or not background rendering is disabled.
    fn is_background_rendering_enabled(&self) -> bool;

    /// Set whether or not object rendering is disabled.
    fn set_object_rendering_enabled(&mut self, enabled: bool);

    /// Get whether or not object rendering is disabled.
    fn is_object_rendering_enabled(&self) -> bool;

    /// Set the sample rate in Hz.
    fn set_sample_rate(&mut self, sample_rate: u32);

    /// Get the sample rate in Hz.
    fn get_sample_rate(&self) -> u32;

    // TODO: set_sample_rate_by_clocks

    /// Set whether or not impossible D-pad inputs are allowed.
    ///
    /// That is: opposite directions like up and down or left and right being pressed simultaneously
    /// is not possible on original hardware due to physical limitations without physical damage on
    /// the console, and a game's programmers most likely have not accounted for that.
    /// 
    /// By default, illegal inputs are **not** allowed.
    fn set_allow_illegal_inputs(&self, allowed: bool);

    /// Save the SRAM / RTC data to a buffer.
    fn save_sram(&self) -> Vec<u8>;

    /// Load the SRAM / RTC data from a buffer.
    fn load_sram(&mut self, sram: &[u8]);

    /// Set the RTC (real time clock) emulation mode.
    fn set_rtc_mode(&mut self, rtc_mode: RtcMode);

    /// Get the CPU registers.
    fn get_registers(&self) -> Registers;

    /// Set the CPU registers.
    fn set_registers(&mut self, registers: &Registers);

    /// Set the maximum rewind length in seconds.
    fn set_rewind_length(&mut self, seconds: f64);

    /// Reset the rewind buffer.
    fn rewind_reset(&mut self);

    /// Set the turbo mode.
    fn set_turbo_mode(&mut self, mode: TurboMode);

    /// Set the RGB encoder.
    /// 
    /// This won't update the current pixel buffer, but future draws will use the new encoder, and
    /// all internal palettes will be updated.
    /// 
    /// The default encoding is [`encode_a8r8g8b8`]. See the [`rgb_encoder`](crate::rgb_encoder)
    /// module for more encoders or supply your own.
    fn set_rgb_encoder(&mut self, encoder: RgbEncoder);

    /// Get the ROM name.
    fn get_rom_title(&self) -> &str;
}


impl RunnableInstanceFunctions for Gameboy {
    #[inline]
    fn get_pixel_buffer_dimensions(&self) -> (u16, u16) {
        self.inner.get_pixel_buffer_dimensions()
    }

    #[inline]
    fn get_pixel_buffer_pixels(&self) -> &[u32] {
        self.inner.get_pixel_buffer_pixels()
    }

    #[inline]
    fn get_pixel_buffer(&self) -> PixelBufferRead<'_> {
        self.inner.get_pixel_buffer()
    }

    #[inline]
    fn set_input_button_state(&mut self, button: InputButton, state: bool) {
        self.do_with_inner_mut(|inner| inner.set_input_button_state(button, state))
    }

    #[inline]
    fn set_input_button_mask(&mut self, state: u8) {
        self.do_with_inner_mut(|inner| inner.set_input_button_mask(state))
    }

    #[inline]
    fn load_boot_rom(&mut self, boot_rom: &[u8]) {
        self.do_with_inner_mut(|inner| inner.load_boot_rom(boot_rom))
    }

    #[inline]
    fn connect_serial(&mut self) {
        self.do_with_inner_mut(|inner| inner.connect_serial())
    }

    #[inline]
    fn connect_printer(&mut self) {
        self.do_with_inner_mut(|inner| inner.connect_printer())
    }

    #[inline]
    fn disconnect_serial(&mut self) {
        self.do_with_inner_mut(|inner| inner.disconnect_serial())
    }

    #[inline]
    fn set_memory_callbacks_enabled(&mut self, enabled: bool) {
        self.do_with_inner_mut(|inner| inner.set_memory_callbacks_enabled(enabled))
    }

    #[inline]
    fn set_rendering_enabled(&mut self, enabled: bool) {
        self.do_with_inner_mut(|inner| inner.set_rendering_enabled(enabled))
    }

    #[inline]
    fn set_clock_multiplier(&mut self, multiplier: f64) {
        self.do_with_inner_mut(|inner| inner.set_clock_multiplier(multiplier))
    }

    #[inline]
    fn get_unmultiplied_clock_rate(&self) -> u32 {
        self.inner.get_unmultiplied_clock_rate()
    }

    #[inline]
    fn get_clock_rate(&self) -> u32 {
        self.inner.get_clock_rate()
    }

    #[inline]
    fn load_rom(&mut self, rom: &[u8]) {
        self.do_with_inner_mut(|inner| inner.load_rom(rom))
    }

    #[inline]
    fn direct_access(&'_ self, access: DirectAccessRegion) -> DirectAccessData<'_> {
        self.inner.direct_access(access)
    }

    #[inline]
    fn direct_access_mut(&'_ mut self, access: DirectAccessRegion) -> DirectAccessDataMut<'_> {
        self.do_with_inner_mut(|inner| unsafe { transmute::<DirectAccessDataMut, DirectAccessDataMut>(inner.direct_access_mut(access)) })
    }

    #[inline]
    fn is_cgb(&self) -> bool {
        self.inner.is_cgb()
    }

    #[inline]
    fn is_cgb_in_cgb_mode(&self) -> bool {
        self.inner.is_cgb_in_cgb_mode()
    }

    #[inline]
    fn is_hle_sgb(&self) -> bool {
        self.inner.is_hle_sgb()
    }

    #[inline]
    fn is_sgb(&self) -> bool {
        self.inner.is_sgb()
    }

    #[inline]
    fn set_border_mode(&mut self, mode: BorderMode) {
        self.do_with_inner_mut(|inner| inner.set_border_mode(mode))
    }

    #[inline]
    fn set_color_correction_mode(&mut self, mode: ColorCorrectionMode) {
        self.do_with_inner_mut(|inner| inner.set_color_correction_mode(mode))
    }

    #[inline]
    fn set_light_temperature(&mut self, temperature: f64) {
        self.do_with_inner_mut(|inner| inner.set_light_temperature(temperature))
    }

    #[inline]
    fn set_palette(&mut self, palette: MonochromePalette) {
        self.do_with_inner_mut(|inner| inner.set_palette(palette))
    }

    #[inline]
    fn get_palette(&self) -> MonochromePalette {
        self.inner.get_palette()
    }

    #[inline]
    fn get_usual_frame_rate(&self) -> f64 {
        self.inner.get_usual_frame_rate()
    }

    #[inline]
    fn is_odd_frame(&self) -> bool {
        self.inner.is_odd_frame()
    }

    #[inline]
    fn set_background_rendering_enabled(&mut self, enabled: bool) {
        self.do_with_inner_mut(|inner| inner.set_background_rendering_enabled(enabled))
    }

    #[inline]
    fn is_background_rendering_enabled(&self) -> bool  {
        self.inner.is_background_rendering_enabled()
    }

    #[inline]
    fn set_object_rendering_enabled(&mut self, enabled: bool) {
        self.do_with_inner_mut(|inner| inner.set_object_rendering_enabled(enabled))
    }

    #[inline]
    fn is_object_rendering_enabled(&self) -> bool  {
        self.inner.is_object_rendering_enabled()
    }

    #[inline]
    fn set_sample_rate(&mut self, sample_rate: u32) {
        self.do_with_inner_mut(|inner| inner.set_sample_rate(sample_rate))
    }

    #[inline]
    fn get_sample_rate(&self) -> u32 {
        self.get_clock_rate()
    }

    #[inline]
    fn set_allow_illegal_inputs(&self, allowed: bool) {
        self.inner.set_allow_illegal_inputs(allowed)
    }

    #[inline]
    fn save_sram(&self) -> Vec<u8> {
        self.inner.save_sram()
    }

    #[inline]
    fn load_sram(&mut self, sram: &[u8]) {
        self.do_with_inner_mut(|inner| inner.load_sram(sram))
    }

    #[inline]
    fn set_rtc_mode(&mut self, rtc_mode: RtcMode) {
        self.do_with_inner_mut(|inner| inner.set_rtc_mode(rtc_mode))
    }

    #[inline]
    fn get_registers(&self) -> Registers {
        self.inner.get_registers()
    }

    #[inline]
    fn set_registers(&mut self, registers: &Registers) {
        self.do_with_inner_mut(|inner| inner.set_registers(registers))
    }

    #[inline]
    fn set_rewind_length(&mut self, seconds: f64) {
        self.do_with_inner_mut(|inner| inner.set_rewind_length(seconds))
    }

    #[inline]
    fn rewind_reset(&mut self) {
        self.do_with_inner_mut(|inner| inner.rewind_reset())
    }

    #[inline]
    fn set_turbo_mode(&mut self, mode: TurboMode) {
        self.do_with_inner_mut(|inner| inner.set_turbo_mode(mode))
    }

    #[inline]
    fn set_rgb_encoder(&mut self, encoder: RgbEncoder) {
        self.do_with_inner_mut(|inner| inner.set_rgb_encoder(encoder))
    }

    #[inline]
    fn get_rom_title(&self) -> &str {
        self.inner.get_rom_title()
    }
}

impl RunnableInstanceFunctions for RunningGameboy {
    #[inline]
    fn get_pixel_buffer_dimensions(&self) -> (u16, u16) {
        (self.screen_width, self.screen_height)
    }

    #[inline]
    fn get_pixel_buffer_pixels(&self) -> &[u32] {
        self.pixel_buffer.as_slice()
    }

    #[inline]
    fn get_pixel_buffer(&self) -> PixelBufferRead<'_> {
        PixelBufferRead { pixels: self.pixel_buffer.as_slice(), width: self.screen_width, height: self.screen_height }
    }

    #[inline]
    fn set_input_button_state(&mut self, button: InputButton, state: bool) {
        unsafe { GB_set_key_state(self.gb, button as u32, state) }
    }

    #[inline]
    fn set_input_button_mask(&mut self, state: u8) {
        unsafe { GB_set_key_mask(self.gb, state as _) }
    }

    #[inline]
    fn load_boot_rom(&mut self, boot_rom: &[u8]) {
        unsafe { GB_load_boot_rom_from_buffer(self.gb, boot_rom.as_ptr(), boot_rom.len()) }
    }

    fn connect_serial(&mut self) {
        unsafe {
            GB_set_serial_transfer_bit_start_callback(self.gb, Some(serial_transfer_bit_start_callback));
            GB_set_serial_transfer_bit_end_callback(self.gb, Some(serial_transfer_bit_end_callback));
        }
    }

    fn connect_printer(&mut self) {
        unsafe {
            GB_connect_printer(self.gb, Some(printer_callback), Some(printer_done_callback));
        }
    }

    fn disconnect_serial(&mut self) {
        unsafe {
            GB_set_serial_transfer_bit_start_callback(self.gb, None);
            GB_set_serial_transfer_bit_end_callback(self.gb, None);
        }
    }

    fn set_memory_callbacks_enabled(&mut self, enabled: bool) {
        unsafe {
            GB_set_read_memory_callback(self.gb, enabled.then_some(read_memory_callback));
            GB_set_write_memory_callback(self.gb, enabled.then_some(write_memory_callback));
            GB_set_execution_callback(self.gb, enabled.then_some(execution_callback));
        }
    }

    #[inline]
    fn set_rendering_enabled(&mut self, enabled: bool) {
        unsafe { GB_set_rendering_disabled(self.gb, !enabled) }
    }

    #[inline]
    fn set_clock_multiplier(&mut self, multiplier: f64) {
        unsafe {
            GB_set_clock_multiplier(self.gb, multiplier)
        }
    }

    #[inline]
    fn get_unmultiplied_clock_rate(&self) -> u32 {
        unsafe {
            GB_get_unmultiplied_clock_rate(self.gb)
        }
    }

    #[inline]
    fn get_clock_rate(&self) -> u32 {
        unsafe {
            GB_get_clock_rate(self.gb)
        }
    }

    #[inline]
    fn load_rom(&mut self, rom: &[u8]) {
        unsafe {
            GB_load_rom_from_buffer(self.gb, rom.as_ptr(), rom.len());
        }
        self.fixup_rom_title();
    }

    fn direct_access(&'_ self, access: DirectAccessRegion) -> DirectAccessData<'_> {
        unsafe {
            direct_access(self.gb, access).into()
        }
    }

    fn direct_access_mut(&'_ mut self, access: DirectAccessRegion) -> DirectAccessDataMut<'_> {
        unsafe {
            direct_access(self.gb, access)
        }
    }

    #[inline]
    fn is_cgb(&self) -> bool {
        unsafe { GB_is_cgb(self.gb) }
    }

    #[inline]
    fn is_cgb_in_cgb_mode(&self) -> bool {
        unsafe { GB_is_cgb_in_cgb_mode(self.gb) }
    }

    #[inline]
    fn is_hle_sgb(&self) -> bool {
        unsafe { GB_is_hle_sgb(self.gb) }
    }

    #[inline]
    fn is_sgb(&self) -> bool {
        unsafe { GB_is_sgb(self.gb) }
    }

    fn set_border_mode(&mut self, mode: BorderMode) {
        unsafe { GB_set_border_mode(self.gb, mode as _) };
        self.reset_pixel_buffer();
    }

    #[inline]
    fn set_color_correction_mode(&mut self, mode: ColorCorrectionMode) {
        unsafe { GB_set_color_correction_mode(self.gb, mode as _) };
    }

    #[inline]
    fn set_light_temperature(&mut self, temperature: f64) {
        unsafe { GB_set_light_temperature(self.gb, temperature) }
    }

    #[inline]
    fn set_palette(&mut self, palette: MonochromePalette) {
        unsafe { GB_set_palette(self.gb, &palette.into_gb()) }
    }

    #[inline]
    fn get_palette(&self) -> MonochromePalette {
        MonochromePalette::from_gb(unsafe { *GB_get_palette(self.gb) })
    }

    #[inline]
    fn get_usual_frame_rate(&self) -> f64 {
        unsafe { GB_get_usual_frame_rate(self.gb) }
    }

    #[inline]
    fn is_odd_frame(&self) -> bool {
        unsafe { GB_is_odd_frame(self.gb) }
    }

    #[inline]
    fn set_background_rendering_enabled(&mut self, enabled: bool) {
        unsafe { GB_set_background_rendering_disabled(self.gb, enabled) }
    }

    #[inline]
    fn is_background_rendering_enabled(&self) -> bool {
        unsafe { GB_is_background_rendering_disabled(self.gb) }
    }

    #[inline]
    fn set_object_rendering_enabled(&mut self, enabled: bool) {
        unsafe { GB_set_object_rendering_disabled(self.gb, enabled) }
    }

    #[inline]
    fn is_object_rendering_enabled(&self) -> bool {
        unsafe { GB_is_object_rendering_disabled(self.gb) }
    }

    #[inline]
    fn set_sample_rate(&mut self, sample_rate: u32) {
        unsafe { GB_set_sample_rate(self.gb, sample_rate as _) }
    }

    #[inline]
    fn get_sample_rate(&self) -> u32 {
        unsafe { GB_get_sample_rate(self.gb) as u32 }
    }

    #[inline]
    fn set_allow_illegal_inputs(&self, allowed: bool) {
        unsafe { GB_set_allow_illegal_inputs(self.gb, allowed) }
    }

    fn save_sram(&self) -> Vec<u8> {
        unsafe {
            let size = usize::try_from(GB_save_battery_size(self.gb)).expect("failed to read save battery size");
            let mut buffer = vec![0u8; size];
            GB_save_battery_to_buffer(self.gb, buffer.as_mut_ptr(), size);
            buffer
        }
    }

    #[inline]
    fn load_sram(&mut self, sram: &[u8]) {
        unsafe { GB_load_battery_from_buffer(self.gb, sram.as_ptr(), sram.len()) }
    }

    #[inline]
    fn set_rtc_mode(&mut self, rtc_mode: RtcMode) {
        unsafe { GB_set_rtc_mode(self.gb, rtc_mode as _) }
    }

    fn get_registers(&self) -> Registers {
        let registers = unsafe { *GB_get_registers(self.gb) };
        unsafe {
            Registers {
                af: registers.__bindgen_anon_1.af,
                bc: registers.__bindgen_anon_1.bc,
                de: registers.__bindgen_anon_1.de,
                hl: registers.__bindgen_anon_1.hl,
                sp: registers.__bindgen_anon_1.sp,
                pc: registers.__bindgen_anon_1.pc,
            }
        }
    }

    fn set_registers(&mut self, registers: &Registers) {
        let registers_out = unsafe { &mut *GB_get_registers(self.gb) };
        registers_out.__bindgen_anon_1.af = registers.af;
        registers_out.__bindgen_anon_1.bc = registers.bc;
        registers_out.__bindgen_anon_1.de = registers.de;
        registers_out.__bindgen_anon_1.hl = registers.hl;
        registers_out.__bindgen_anon_1.sp = registers.sp;
        registers_out.__bindgen_anon_1.pc = registers.pc;
    }

    #[inline]
    fn set_rewind_length(&mut self, seconds: f64) {
        unsafe { GB_set_rewind_length(self.gb, seconds) }
    }

    #[inline]
    fn rewind_reset(&mut self) {
        unsafe { GB_rewind_reset(self.gb) }
    }

    fn set_turbo_mode(&mut self, mode: TurboMode) {
        match mode {
            TurboMode::Disabled => unsafe { GB_set_turbo_mode(self.gb, false, true) },
            TurboMode::EnabledFrameSkipped => unsafe { GB_set_turbo_mode(self.gb, true, false) },
            TurboMode::Enabled => unsafe { GB_set_turbo_mode(self.gb, true, true) }
        }
    }

    #[inline]
    fn set_rgb_encoder(&mut self, encoder: RgbEncoder) {
        self.rgb_encoder = encoder;

        // changing the encoder has the side effect of telling SameBoy to update the palettes
        unsafe { GB_set_rgb_encode_callback(self.gb, Some(rgb_encode_callback)) };
    }

    #[inline]
    fn get_rom_title(&self) -> &str {
        self.rom_title.as_str()
    }
}

/// Represents all registers packed into 16-bit values.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Registers {
    /// High = A, Low = flags
    pub af: u16,

    /// High = B, Low = C
    pub bc: u16,

    /// High = D, Low = E
    pub de: u16,

    /// High = H, Low = L
    pub hl: u16,

    /// Stack pointer
    pub sp: u16,

    /// Program counter
    pub pc: u16
}

/// Input buttons mapped to a Game Boy.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
pub enum InputButton {
    /// D-Pad right
    Right = sameboy_sys::GB_key_t_GB_KEY_RIGHT,

    /// D-Pad left
    Left = sameboy_sys::GB_key_t_GB_KEY_LEFT,

    /// D-Pad up
    Up = sameboy_sys::GB_key_t_GB_KEY_UP,

    /// D-Pad down
    Down = sameboy_sys::GB_key_t_GB_KEY_DOWN,

    /// A button
    A = sameboy_sys::GB_key_t_GB_KEY_A,

    /// B button
    B = sameboy_sys::GB_key_t_GB_KEY_B,

    /// Select button
    Select = sameboy_sys::GB_key_t_GB_KEY_SELECT,

    /// Start button
    Start = sameboy_sys::GB_key_t_GB_KEY_START
}

impl Shl<InputButton> for u8 {
    type Output = u8;

    fn shl(self, rhs: InputButton) -> Self::Output {
        self << (rhs as u8)
    }
}

impl ShlAssign<InputButton> for u8 {
    fn shl_assign(&mut self, rhs: InputButton) {
        *self = *self << rhs;
    }
}

/// SNES/SFC border mode.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
pub enum BorderMode {
    /// Only draw a border when in SGB mode (default).
    SgbOnly = sameboy_sys::GB_border_mode_t_GB_BORDER_SGB,

    /// Never draw a border.
    Never = sameboy_sys::GB_border_mode_t_GB_BORDER_NEVER,

    /// Always draw a border.
    Always = sameboy_sys::GB_border_mode_t_GB_BORDER_ALWAYS
}

/// Specifies a region to directly access.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
#[allow(missing_docs)]
pub enum DirectAccessRegion {
    ROM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_ROM,
    RAM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_RAM,
    CartRAM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_CART_RAM,
    VRAM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_VRAM,
    HRAM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_HRAM,
    IO = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_IO,
    BootROM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_BOOTROM,
    OAM = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_OAM,
    BGP = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_BGP,
    OBP = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_OBP,
    IE = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_IE,
    ROM0 = sameboy_sys::GB_direct_access_t_GB_DIRECT_ACCESS_ROM0,
}

/// Specifies color correction.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u32)]
pub enum ColorCorrectionMode {
    /// Color correction is disabled, and colors are mapped directly to sRGB.
    Disabled = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_DISABLED,

    /// Brightness is corrected, but hues are not.
    CorrectCurves = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_CORRECT_CURVES,

    /// Brightness and hues are corrected for a modern display.
    ModernAccurate = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_ACCURATE,

    /// Brightness and hues are corrected for a modern display, and blue contrast is boosted.
    ModernBalanced = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BALANCED,

    /// Contrast is boosted beyond [`ColorCorrectionMode::ModernBalanced`].
    ModernBoostContrast = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_MODERN_BOOST_CONTRAST,

    /// Contrast is reduced to better match the original display.
    ReduceContrast = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_REDUCE_CONTRAST,

    /// Heavily reduced contrast.
    LowContrast = sameboy_sys::GB_color_correction_mode_t_GB_COLOR_CORRECTION_LOW_CONTRAST,
}

/// Pixels and all dimensions for a screen buffer.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct PixelBufferRead<'a> {
    /// Reference to the contents of the current pixel buffer.
    pub pixels: &'a [u32],

    /// Width of the pixel buffer in pixels.
    pub width: u16,

    /// Height of the pixel buffer in pixels.
    pub height: u16
}

/// A region of memory in a Game Boy.
pub struct DirectAccessData<'a> {
    /// Pointer to the data.
    pub data: &'a [u8],

    /// Current bank, if any.
    pub bank: u16
}

/// A region of memory in a Game Boy.
pub struct DirectAccessDataMut<'a> {
    /// Pointer to the data.
    pub data: &'a mut [u8],

    /// Current bank, if any.
    pub bank: u16
}

impl<'a> From<DirectAccessDataMut<'a>> for DirectAccessData<'a> {
    fn from(value: DirectAccessDataMut<'a>) -> Self {
        Self {
            data: value.data,
            bank: value.bank
        }
    }
}

/// Specifies a monochrome palette for setting/getting from the emulator.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MonochromePalette {
    /// Colors in `r, g, b` order.
    ///
    /// The first four colors are in order of darkest to lightest, and the uppermost color is the
    /// "off" color.
    pub rgb: [(u8, u8, u8); 5]
}

impl MonochromePalette {
    const fn from_gb(palette: GB_palette_t) -> Self {
        Self {
            rgb: [
                (palette.colors[0].r, palette.colors[0].g, palette.colors[0].b),
                (palette.colors[1].r, palette.colors[1].g, palette.colors[1].b),
                (palette.colors[2].r, palette.colors[2].g, palette.colors[2].b),
                (palette.colors[3].r, palette.colors[3].g, palette.colors[3].b),
                (palette.colors[4].r, palette.colors[4].g, palette.colors[4].b),
            ]
        }
    }
    const fn into_gb(self) -> GB_palette_t {
        GB_palette_t {
            colors: [
                GB_palette_t_GB_color_s { r: self.rgb[0].0, g: self.rgb[0].1, b: self.rgb[0].2},
                GB_palette_t_GB_color_s { r: self.rgb[1].0, g: self.rgb[1].1, b: self.rgb[1].2},
                GB_palette_t_GB_color_s { r: self.rgb[2].0, g: self.rgb[2].1, b: self.rgb[2].2},
                GB_palette_t_GB_color_s { r: self.rgb[3].0, g: self.rgb[3].1, b: self.rgb[3].2},
                GB_palette_t_GB_color_s { r: self.rgb[4].0, g: self.rgb[4].1, b: self.rgb[4].2},
            ]
        }
    }
}

/// Specifies an RTC mode for timing the real-time clock.
#[repr(u32)]
pub enum RtcMode {
    /// One second in the emulator is one second on the host.
    ///
    /// This applies even if the emulator is not running at 1x speed or is paused.
    SyncToHost = sameboy_sys::GB_rtc_mode_t_GB_RTC_MODE_SYNC_TO_HOST,

    /// The RTC timings are accurately emulated.
    Accurate = sameboy_sys::GB_rtc_mode_t_GB_RTC_MODE_ACCURATE,
}

/// Specifies a turbo mode for controlling the timing of the emulator.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TurboMode {
    /// Turbo mode is disabled.
    Disabled,

    /// Turbo mode is enabled.
    ///
    /// The vblank callback will be limited to 60 FPS.
    EnabledFrameSkipped,

    /// Turbo mode is enabled.
    ///
    /// The vblank callback will be called on vblank.
    Enabled
}

unsafe fn direct_access(gb: *mut GB_gameboy_t, access: DirectAccessRegion) -> DirectAccessDataMut<'static> {
    let mut bank = 0u16;
    let mut size = 0usize;
    let data = unsafe {
        let ptr = GB_get_direct_access(gb, access as _, &mut size, &mut bank) as *mut u8;
        core::slice::from_raw_parts_mut(ptr, size)
    };
    DirectAccessDataMut {
        data, bank
    }
}
