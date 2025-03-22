pub mod event;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;
use core::mem::zeroed;
use alloc::string::{String, ToString};
use core::ffi::*;
use core::marker::PhantomPinned;
use sameboy_sys::*;
use crate::gb::event::Event;
use crate::types::*;
use event::inner::*;

pub struct Gameboy {
    inner: Box<GameboyStateInner>
}

impl Gameboy {
    pub fn new(model: Model, rgb_encoding: RgbEncoding, enabled_events: EnabledEvents) -> Self {
        unsafe {
            let gb = GB_alloc();
            let mut inner = Box::new(GameboyStateInner {
                gb,
                pixel_buffer: Vec::new(),
                rendering_disabled: false,
                user_data: None,
                rgb_encoding,
                enabled_events,
                events: Vec::with_capacity(1024),
                read_memory_callback: None,
                write_memory_callback: None,
                running: false,
                _phantom_pinned: Default::default()
            });

            GB_init(gb, model as GB_model_t);
            GB_set_rendering_disabled(gb, true);
            GB_set_user_data(gb, inner.as_mut() as *mut GameboyStateInner as *mut _);

            // set dummy callbacks in case the user forgets to specify any
            GB_set_rgb_encode_callback(gb, Some(rgb_encode_callback));
            GB_apu_set_sample_callback(gb, Some(sample_callback));
            GB_set_rumble_callback(gb, Some(rumble_callback));
            GB_set_vblank_callback(gb, Some(vblank_callback));

            GB_set_read_memory_callback(gb, Some(read_memory_callback));
            GB_set_write_memory_callback(gb, Some(write_memory_callback));

            Self { inner }
        }
    }

    /// Iterate through all events thus far.
    pub fn iter_events<'a>(&'a mut self) -> impl Iterator<Item = Event> + 'a {
        self.inner.events.drain(..)
    }

    /// Get all enabled events.
    pub fn get_enabled_events(&self) -> EnabledEvents {
        self.inner.enabled_events
    }

    /// Set all enabled events.
    pub fn set_enabled_events(&mut self, enabled_events: EnabledEvents) {
        self.inner.enabled_events = enabled_events;
    }

    /// Get the model to use for the given save state.
    ///
    /// Returns `Err` if the save state is invalid or a model couldn't be determined.
    pub fn model_for_save_state(save_state: &[u8]) -> Result<Model, ()> {
        let mut model = 0 as GB_model_t;
        let is_ok = unsafe { GB_get_state_model_from_buffer(save_state.as_ptr(), save_state.len(), &mut model) };

        if is_ok != 0 {
            Err(())
        }
        else {
            (model as u32).try_into()
        }
    }

    /// Sets the data which will be passed to callbacks.
    pub fn set_user_data(&mut self, data: Option<Box<dyn Any>>) {
        self.inner.user_data = data
    }

    /// Get the user data, if any.
    pub fn get_user_data(&mut self) -> Option<&mut dyn Any> {
        self.inner.user_data.as_mut().map(|b| b.as_mut())
    }

    /// Set the model to `model` and reset the emulator.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn switch_model_and_reset(&mut self, model: Model) {
        self.assert_not_running();
        unsafe { GB_switch_model_and_reset(self.inner.gb, model as GB_model_t) };
        self.inner.reset_pixel_buffer();
    }

    /// Hard reset the emulator.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn reset(&mut self) {
        self.assert_not_running();
        unsafe { GB_reset(self.inner.gb) }
    }

    /// Reset the emulator, but retain HRAM, tile data, object memory, palette data, and DMA state.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn partial_reset(&mut self) {
        self.assert_not_running();
        unsafe { GB_quick_reset(self.inner.gb) }
    }

    /// Inverts the GB camera flags.
    pub fn camera_updated(&mut self) {
        unsafe { GB_camera_updated(self.inner.gb) }
    }

    /// Connect an emulated printer.
    ///
    /// Disable with [disconnect_serial](Self::disconnect_serial).
    ///
    /// While enabled, printer events will be sent if EnabledEvents::printer is set.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn connect_printer(&mut self) {
        self.assert_not_running();
        unsafe { GB_connect_printer(self.inner.gb, Some(printer_callback), Some(printer_done_callback)) }
    }

    // pub fn connect_workboy(&mut self) {
    //     unsafe { GB_connect_workboy(self.inner.gb) }
    // }

    /// Convert R5G5B5 to 32-bit color.
    pub fn convert_rgb15(&self, color: u16, for_border: bool) -> u32 {
        unsafe { GB_convert_rgb15(self.inner.gb, color, for_border) }
    }

    /// Disconnect the link cable.
    pub fn disconnect_serial(&mut self) {
        unsafe { GB_disconnect_serial(self.inner.gb) }
    }

    // pub fn draw_tilemap(&mut self) {
    //     unsafe { GB_draw_tilemap(self.inner.gb) }
    // }
    //
    // pub fn draw_tileset(&mut self) {
    //     unsafe { GB_draw_tileset(self.inner.gb) }
    // }

    /// Switch the current track.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn gbs_switch_track(&mut self, track: u8) {
        self.assert_not_running();
        unsafe { GB_gbs_switch_track(self.inner.gb, track) }
    }

    /// Load a GBS from the slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn load_gbs_from_slice(&mut self, buffer: &[u8]) -> Result<GBSInfo, &'static str> {
        self.assert_not_running();

        let mut info: GB_gbs_info_t = unsafe { zeroed() };
        let success = unsafe { GB_load_gbs_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len(), &mut info) } == 0;

        if !success {
            return Err("invalid GBS")
        }

        Ok(info.into())
    }

    // pub fn get_apu_wave_table(&mut self) {
    //     unsafe { GB_get_apu_wave_table(self.inner.gb) }
    // }

    /// Get the current accessory if one is plugged in.
    pub fn get_built_in_accessory(&self) -> Accessory {
        unsafe { GB_get_built_in_accessory(self.inner.gb) }.into()
    }

    /// Get the channel amplitude.
    pub fn get_channel_amplitude(&self, channel: AudioChannel) -> u8 {
        unsafe { GB_get_channel_amplitude(self.inner.gb, channel as GB_channel_t) }
    }

    /// Return `true` if a channel edge is triggered.
    pub fn get_channel_edge_triggered(&self, channel: AudioChannel) -> bool {
        unsafe { GB_get_channel_edge_triggered(self.inner.gb, channel as GB_channel_t) }
    }

    /// Get the period of the audio channel.
    pub fn get_channel_period(&mut self, channel: AudioChannel) -> u16 {
        unsafe { GB_get_channel_period(self.inner.gb, channel as GB_channel_t) }
    }

    /// Set the volume for a given audio channel.
    pub fn get_channel_volume(&mut self, channel: AudioChannel) -> u8 {
        unsafe { GB_get_channel_volume(self.inner.gb, channel as GB_channel_t) }
    }

    /// Return `true` if a given audio channel is muted.
    pub fn is_channel_muted(&self, channel: AudioChannel) -> bool {
        unsafe { GB_is_channel_muted(self.inner.gb, channel as GB_channel_t) }
    }

    /// Mute a given audio channel.
    pub fn set_channel_muted(&mut self, channel: AudioChannel, muted: bool) {
        unsafe { GB_set_channel_muted(self.inner.gb, channel as GB_channel_t, muted) }
    }

    /// Get the effective clock rate.
    pub fn get_clock_rate(&self) -> u32 {
        unsafe { GB_get_clock_rate(self.inner.gb) }
    }

    /// Returns a mutable reference to the data as well as the bank.
    pub fn get_direct_access_mut(&mut self, access: DirectAccess) -> (&mut [u8], u16) {
        self.get_direct_access_inner(access)
    }

    /// Returns a reference to the data as well as the bank.
    pub fn get_direct_access(&self, access: DirectAccess) -> (&[u8], u16) {
        let (r, bank) = self.get_direct_access_inner(access);
        (r, bank)
    }

    fn get_direct_access_inner(&self, access: DirectAccess) -> (&mut [u8], u16) {
        // Since it's technically 'static, we can just have one method for accessing the data, and
        // we can change it to an immutable reference later.
        let mut size: usize = 0;
        let mut bank: u16 = 0;

        let access = unsafe { GB_get_direct_access(self.inner.gb, access as GB_direct_access_t, &mut size, &mut bank) };

        if access.is_null() {
            (&mut [], bank)
        }
        else {
            // SAFETY: GB_get_direct_access's size value corresponds to the pointer it returns
            (unsafe { core::slice::from_raw_parts_mut(access as *mut u8, size) }, bank)
        }
    }

    /// Return `true` if JOYP was accessed.
    pub fn get_joyp_accessed(&self) -> bool {
        unsafe { GB_get_joyp_accessed(self.inner.gb) }
    }

    /// Clear the flag for if JOYP was accessed.
    pub fn clear_joyp_accessed(&mut self) {
        unsafe { GB_clear_joyp_accessed(self.inner.gb) }
    }

    /// Get the currently loaded model.
    pub fn get_model(&self) -> Model {
        let model = unsafe { GB_get_model(self.inner.gb) };
        let Ok(model) = Model::try_from(model) else {
            unreachable!("unknown model {model:?}")
        };
        model
    }

    // pub fn get_oam_info(&mut self) {
    //     unsafe { GB_get_oam_info(self.inner.gb) }
    // }

    // pub fn get_palette(&mut self) {
    //     unsafe { GB_get_palette(self.inner.gb) }
    // }

    /// Get player count (relevant for SGB).
    pub fn get_player_count(&mut self) -> u32 {
        unsafe { GB_get_player_count(self.inner.gb) }
    }

    /// Get the current state of the registers.
    pub fn get_registers(&self) -> Registers {
        unsafe {
            let registers = GB_get_registers(self.inner.gb);
            (*registers).into()
        }
    }

    /// Get the CRC32 of the ROM.
    pub fn get_rom_crc32(&mut self) -> u32 {
        unsafe { GB_get_rom_crc32(self.inner.gb) }
    }

    /// Get the ROM title from the cartridge header.
    pub fn get_rom_title(&mut self) -> String {
        let mut title = [0u8; 17];
        unsafe { GB_get_rom_title(self.inner.gb, &mut title as *mut u8 as *mut c_char) };

        let c_str = CStr::from_bytes_until_nul(&title)
            .expect("should be null terminated or else SameBoy messed up...");

        c_str.to_string_lossy().to_string()
    }

    /// Get the current audio sample rate.
    pub fn get_sample_rate(&self) -> u32 {
        unsafe { GB_get_sample_rate(self.inner.gb) }
    }

    /// Get the height of the screen in pixels.
    pub fn get_screen_height(&self) -> usize {
        self.inner.get_screen_height()
    }

    /// Get the width of the screen in pixels.
    pub fn get_screen_width(&self) -> usize {
        self.inner.get_screen_width()
    }

    /// Get the base clock rate in clocks per second (Hz).
    pub fn get_unmultiplied_clock_rate(&self) -> u32 {
        unsafe { GB_get_unmultiplied_clock_rate(self.inner.gb) }
    }

    /// Get the base frame rate in frames per second.
    pub fn get_usual_frame_rate(&self) -> f64 {
        unsafe { GB_get_usual_frame_rate(self.inner.gb) }
    }

    /// Return `true` if an accelerometer is present.
    pub fn has_accelerometer(&self) -> bool {
        unsafe { GB_has_accelerometer(self.inner.gb) }
    }

    pub fn icd_set_joyp(&mut self, value: u8) {
        unsafe { GB_icd_set_joyp(self.inner.gb, value) }
    }

    /// Return `true` if background rendering is disabled.
    pub fn is_background_rendering_disabled(&self) -> bool {
        unsafe { GB_is_background_rendering_disabled(self.inner.gb) }
    }

    /// Return `true` if a Game Boy Color instance.
    pub fn is_cgb(&self) -> bool {
        unsafe { GB_is_cgb(self.inner.gb) }
    }

    /// Return `true` if a Game Boy Color instance running a Game Boy Color game.
    pub fn is_cgb_in_cgb_mode(&self) -> bool {
        unsafe { GB_is_cgb_in_cgb_mode(self.inner.gb) }
    }

    /// Return `true` if SGB with HLE.
    pub fn is_hle_sgb(&self) -> bool {
        unsafe { GB_is_hle_sgb(self.inner.gb) }
    }

    /// Return `true` if object rendering is disabled.
    pub fn is_object_rendering_disabled(&self) -> bool {
        unsafe { GB_is_object_rendering_disabled(self.inner.gb) }
    }

    /// Return `true` if the frame is odd.
    pub fn is_odd_frame(&self) -> bool {
        unsafe { GB_is_odd_frame(self.inner.gb) }
    }

    /// Return `true` if SGB.
    pub fn is_sgb(&self) -> bool {
        unsafe { GB_is_sgb(self.inner.gb) }
    }

    /// Load SRAM from the slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn load_sram_from_slice(&mut self, buffer: &[u8]) {
        self.assert_not_running();
        unsafe { GB_load_battery_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) }
    }

    /// Load a boot ROM from the slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn load_boot_rom_from_slice(&mut self, buffer: &[u8]) {
        self.assert_not_running();
        unsafe { GB_load_boot_rom_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) }
    }

    /// Load a ROM from the slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn load_rom_from_slice(&mut self, buffer: &[u8]) {
        self.assert_not_running();
        unsafe { GB_load_rom_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) }
    }

    /// Load a save state from the slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn load_state_from_slice(&mut self, buffer: &[u8]) -> Result<(), ()> {
        let result = unsafe { GB_load_state_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) };
        if result != 0 {
            Err(())
        }
        else {
            Ok(())
        }
    }

    /// Read memory at the given address.
    ///
    /// This may trigger callbacks.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn read_memory(&mut self, addr: u16) -> u8 {
        self.assert_not_running();
        unsafe { GB_read_memory(self.inner.gb, addr) }
    }

    /// Rewind to the previous state on the stack.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn rewind_pop(&mut self) -> bool {
        self.assert_not_running();
        unsafe { GB_rewind_pop(self.inner.gb) }
    }

    /// Clear all rewind states.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn rewind_reset(&mut self) {
        self.assert_not_running();
        unsafe { GB_rewind_reset(self.inner.gb) }
    }

    /// Set the rewind state buffer length in seconds.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn set_rewind_length(&mut self, seconds: f64) {
        self.assert_not_running();
        unsafe { GB_set_rewind_length(self.inner.gb, seconds) }
    }

    pub fn rom_supports_alarms(&mut self) -> bool {
        unsafe { GB_rom_supports_alarms(self.inner.gb) }
    }

    pub fn time_to_alarm(&mut self) -> u32 {
        unsafe { GB_time_to_alarm(self.inner.gb) as u32 }
    }

    /// Return `true` if the emulator is currently running. If so, some functions cannot be called.
    ///
    /// This will generally be the case if inside a callback.
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    /// Run for a few cycles.
    ///
    /// Returns the number of 8 MiHz cycles that passed.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn run(&mut self) -> u64 {
        self.assert_not_running();
        self.inner.running = true;
        let result = unsafe { GB_run(self.inner.gb) as u64 };
        self.inner.running = false;
        result
    }

    /// Run for one frame.
    ///
    /// Returns the number of nanoseconds passed since the last frame.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn run_frame(&mut self) -> u64 {
        self.assert_not_running();
        self.inner.running = true;
        let result = unsafe { GB_run_frame(self.inner.gb) };
        self.inner.running = false;
        result
    }

    /// Read memory at the address.
    ///
    /// NOTE: This will still trigger the read_memory_callback.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn safe_read_memory(&mut self, addr: u16) -> u8 {
        self.assert_not_running();
        unsafe { GB_safe_read_memory(self.inner.gb, addr) }
    }

    /// Get the SRAM size in bytes.
    pub fn get_sram_size(&self) -> usize {
        (unsafe { GB_save_battery_size(self.inner.gb) }) as usize
    }

    /// Write SRAM to a slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()` or `data.len() != self.get_sram_size()`
    pub fn read_sram_to_slice(&self, data: &mut [u8]) {
        self.assert_not_running();
        assert_eq!(data.len(), self.get_sram_size());
        unsafe {
            GB_save_battery_to_buffer(self.inner.gb, data.as_mut_ptr(), data.len());
        }
    }

    /// Read SRAM to a vector.
    ///
    /// `vec` will be cleared, with its contents replaced by the contents of the save state.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn read_sram_to_vec(&self, vec: &mut Vec<u8>) {
        let len = self.get_sram_size();
        vec.clear();
        vec.reserve_exact(len);
        unsafe {
            vec.set_len(len);
        }
        self.read_sram_to_slice(vec.as_mut_slice());
    }

    /// Get the size of a save state in bytes.
    pub fn get_save_state_size(&self) -> usize {
        unsafe { GB_get_save_state_size(self.inner.gb) }
    }

    /// Read the save state to a slice.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()` or `data.len() != self.get_save_state_size()`
    pub fn read_save_state_to_slice(&self, data: &mut [u8]) {
        self.assert_not_running();
        assert_eq!(data.len(), self.get_save_state_size());
        unsafe {
            GB_save_state_to_buffer(self.inner.gb, data.as_mut_ptr());
        }
    }

    /// Read the save state to a vector.
    ///
    /// `vec` will be cleared, with its contents replaced by the contents of the save state.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn read_save_state_to_vec(&self, vec: &mut Vec<u8>) {
        let len = self.get_save_state_size();
        vec.clear();
        vec.reserve_exact(len);
        unsafe {
            vec.set_len(len);
        }
        self.read_save_state_to_slice(vec.as_mut_slice());
    }

    // TODO: CHEATS
    //
    // pub fn get_cheats(&mut self) {
    //     unsafe { GB_get_cheats(self.inner.gb) }
    // }
    //
    // pub fn import_cheat(&mut self) {
    //     unsafe { GB_import_cheat(self.inner.gb) }
    // }
    //
    // pub fn load_cheats(&mut self) {
    //     unsafe { GB_load_cheats(self.inner.gb) }
    // }
    //
    // pub fn remove_cheat(&mut self) {
    //     unsafe { GB_remove_cheat(self.inner.gb) }
    // }
    //
    // pub fn save_cheats(&mut self) {
    //     unsafe { GB_save_cheats(self.inner.gb) }
    // }
    //
    // pub fn cheats_enabled(&mut self) -> bool {
    //     unsafe { GB_cheats_enabled(self.inner.gb) }
    // }
    //
    // pub fn add_cheat(&mut self) {
    //     self.assert_not_running();
    //     unsafe { GB_add_cheat(self.inner.gb) }
    // }
    //
    // pub fn update_cheat(&mut self) {
    //     self.assert_not_running();
    //     unsafe { GB_update_cheat(self.inner.gb) }
    // }
    //
    // pub fn set_cheats_enabled(&mut self) {
    //     unsafe { GB_set_cheats_enabled(self.inner.gb) }
    // }

    pub fn serial_get_data_bit(&self) -> bool {
        unsafe { GB_serial_get_data_bit(self.inner.gb) }
    }

    pub fn serial_set_data_bit(&mut self, data: bool) {
        unsafe { GB_serial_set_data_bit(self.inner.gb, data) }
    }

    pub fn set_accelerometer_values(&mut self, x: f64, y: f64) {
        unsafe { GB_set_accelerometer_values(self.inner.gb, x, y) }
    }

    pub fn set_allow_illegal_inputs(&mut self, allowed: bool) {
        unsafe { GB_set_allow_illegal_inputs(self.inner.gb, allowed) }
    }

    pub fn set_background_rendering_disabled(&mut self, disabled: bool) {
        unsafe { GB_set_background_rendering_disabled(self.inner.gb, disabled) }
    }

    pub fn set_border_mode(&mut self, mode: SGBBorderMode) {
        unsafe { GB_set_border_mode(self.inner.gb, mode as GB_border_mode_t) }
    }

    /// Set the emulation speed ratio.
    ///
    /// For example, 1.0 = 100% speed, 2.0 = 200% speed, etc.
    ///
    /// # Panics
    ///
    /// Panics if `multiplier <= 0.0`.
    pub fn set_clock_multiplier(&mut self, multiplier: f64) {
        assert!(multiplier > 0.0);
        unsafe { GB_set_clock_multiplier(self.inner.gb, multiplier) }
    }

    pub fn set_color_correction_mode(&mut self, mode: ColorCorrectionMode) {
        unsafe { GB_set_color_correction_mode(self.inner.gb, mode as GB_color_correction_mode_t) }
    }

    pub fn set_emulate_joypad_bouncing(&mut self, emulate: bool) {
        unsafe { GB_set_emulate_joypad_bouncing(self.inner.gb, emulate) }
    }

    pub fn set_highpass_filter_mode(&mut self, mode: HighpassFilterMode) {
        unsafe { GB_set_highpass_filter_mode(self.inner.gb, mode as GB_highpass_mode_t) }
    }

    pub fn set_infrared_input(&mut self, state: bool) {
        unsafe { GB_set_infrared_input(self.inner.gb, state) }
    }

    pub fn set_interference_volume(&mut self, volume: f64) {
        unsafe { GB_set_interference_volume(self.inner.gb, volume) }
    }

    pub fn set_key_mask(&mut self, mask: u8) {
        unsafe { GB_set_key_mask(self.inner.gb, mask as GB_key_mask_t) }
    }

    pub fn set_key_mask_for_player(&mut self, mask: u8, player: u8) {
        unsafe { GB_set_key_mask_for_player(self.inner.gb, mask as GB_key_mask_t, player as c_uint) }
    }

    pub fn set_key_state(&mut self, key: Key, state: bool) {
        unsafe { GB_set_key_state(self.inner.gb, key as GB_key_t, state) }
    }

    pub fn set_key_state_for_player(&mut self, key: Key, player: u8, state: bool) {
        unsafe { GB_set_key_state_for_player(self.inner.gb, key as GB_key_t, player as c_uint, state) }
    }

    pub fn set_light_temperature(&mut self, temperature: f64) {
        unsafe { GB_set_light_temperature(self.inner.gb, temperature) }
    }

    pub fn set_object_rendering_disabled(&mut self, disabled: bool) {
        unsafe { GB_set_object_rendering_disabled(self.inner.gb, disabled) }
    }

    pub fn set_open_bus_decay_time(&mut self, decay: u32) {
        unsafe { GB_set_open_bus_decay_time(self.inner.gb, decay) }
    }

    pub fn set_palette(&mut self, palette: Palette) {
        let palette = GB_palette_t {
            colors: core::array::from_fn(|f| palette[f].into())
        };
        unsafe { GB_set_palette(self.inner.gb, &palette) }
    }

    /// Set whether rendering is disabled.
    ///
    /// Note that vblank events will still be recorded in events, but the pixel buffer won't be
    /// updated. This can be useful for improving performance if a graphical output is not desired.
    pub fn set_rendering_disabled(&mut self, disabled: bool) {
        unsafe { GB_set_rendering_disabled(self.inner.gb, disabled) };
        self.inner.rendering_disabled = disabled;
        self.inner.reset_pixel_buffer();
    }

    /// Get the current pixel buffer.
    pub fn get_pixel_buffer(&self) -> &[u32] {
        self.inner.pixel_buffer.as_slice()
    }

    /// Set the real-time clock mode.
    pub fn set_rtc_mode(&mut self, mode: RTCMode) {
        unsafe { GB_set_rtc_mode(self.inner.gb, mode as GB_rtc_mode_t) }
    }

    /// Set the real-time multiplier.
    pub fn set_rtc_multiplier(&mut self, multiplier: f64) {
        unsafe { GB_set_rtc_multiplier(self.inner.gb, multiplier) }
    }

    /// Set the rumble mode.
    ///
    /// Rumble events will be sent to Events if enabled and EnabledEvents::rumble is set.
    pub fn set_rumble_mode(&mut self, mode: Rumble) {
        unsafe { GB_set_rumble_mode(self.inner.gb, mode as GB_rumble_mode_t) }
    }

    /// Set the audio sample rate.
    ///
    /// Audio events will be sent to Events if non-zero and EnabledEvents::sample is set.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        unsafe { GB_set_sample_rate(self.inner.gb, sample_rate as c_uint) }
    }

    /// Set the audio sample rate based on clock rate.
    ///
    /// Audio events will be sent to Events if non-zero and EnabledEvents::sample is set.
    pub fn set_sample_rate_by_clocks(&mut self, clocks_per_sample: f64) {
        unsafe { GB_set_sample_rate_by_clocks(self.inner.gb, clocks_per_sample) }
    }

    /// Set turbo mode.
    ///
    /// If enabled, the emulator will run at an uncapped frame rate.
    pub fn set_turbo_mode(&mut self, turbo: bool, no_frame_skip: bool) {
        unsafe { GB_set_turbo_mode(self.inner.gb, turbo, no_frame_skip) }
    }

    /// Write the memory to the given address.
    ///
    /// This may have side effects.
    ///
    /// # Panics
    ///
    /// Panics if `self.is_running()`
    pub fn write_memory(&mut self, addr: u16, value: u8) {
        self.assert_not_running();
        unsafe { GB_write_memory(self.inner.gb, addr, value) }
    }

    /// Run the callback when memory is being read.
    ///
    /// Unlike with `track_memory_reads`, you can intercept the read and even change it by returning a different byte.
    pub fn set_read_memory_callback(&mut self, callback: Option<ReadMemoryCallback>) {
        self.inner.read_memory_callback = callback;
    }

    /// Run the callback when memory is being written.
    ///
    /// Unlike with `track_memory_writes`, you can intercept the write and even prevent it by returning `false`.
    pub fn set_write_memory_callback(&mut self, callback: Option<WriteMemoryCallback>) {
        self.inner.write_memory_callback = callback;
    }

    // TODO: A few callbacks
    //
    // pub fn set_joyp_write_callback(&mut self) {
    //     unsafe { GB_set_joyp_write_callback(self.inner.gb) }
    // }
    //
    // pub fn set_boot_rom_load_callback(&mut self) {
    //     unsafe { GB_set_boot_rom_load_callback(self.inner.gb) }
    // }
    //
    // pub fn set_icd_hreset_callback(&mut self) {
    //     unsafe { GB_set_icd_hreset_callback(self.inner.gb) }
    // }
    //
    // pub fn set_icd_pixel_callback(&mut self) {
    //     unsafe { GB_set_icd_pixel_callback(self.inner.gb) }
    // }
    //
    // pub fn set_icd_vreset_callback(&mut self) {
    //     unsafe { GB_set_icd_vreset_callback(self.inner.gb) }
    // }
    //
    // pub fn set_infrared_callback(&mut self) {
    //     unsafe { GB_set_infrared_callback(self.inner.gb) }
    // }
    //
    // pub fn set_lcd_line_callback(&mut self) {
    //     unsafe { GB_set_lcd_line_callback(self.inner.gb) }
    // }
    //
    // pub fn set_lcd_status_callback(&mut self) {
    //     unsafe { GB_set_lcd_status_callback(self.inner.gb) }
    // }
    //
    // pub fn set_serial_transfer_bit_end_callback(&mut self) {
    //     unsafe { GB_set_serial_transfer_bit_end_callback(self.inner.gb) }
    // }
    //
    // pub fn set_serial_transfer_bit_start_callback(&mut self) {
    //     unsafe { GB_set_serial_transfer_bit_start_callback(self.inner.gb) }
    // }
    //
    // pub fn set_update_input_hint_callback(&mut self) {
    //     unsafe { GB_set_update_input_hint_callback(self.inner.gb) }
    // }

    // TODO: GB Camera
    //
    // pub fn set_camera_get_pixel_callback(&mut self) {
    //     unsafe { GB_set_camera_get_pixel_callback(self.inner.gb) }
    // }
    //
    // pub fn set_camera_update_request_callback(&mut self) {
    //     unsafe { GB_set_camera_update_request_callback(self.inner.gb) }
    // }

    // TODO: Audio recording?
    //
    // pub fn start_audio_recording(&mut self) {
    //     unsafe { GB_start_audio_recording(self.inner.gb) }
    // }
    //
    // pub fn stop_audio_recording(&mut self) {
    //     unsafe { GB_start_audio_recording(self.inner.gb) }
    // }

    // TODO: Workboy?
    //
    // pub fn workboy_is_enabled(&mut self) {
    //     unsafe { GB_workboy_is_enabled(self.inner.gb) }
    // }
    //
    // pub fn workboy_set_key(&mut self) {
    //     unsafe { GB_workboy_set_key(self.inner.gb) }
    // }

    // TODO: Debugger
    //
    // pub fn set_debugger_reload_callback(&mut self) {
    //     unsafe { GB_set_debugger_reload_callback(self.inner.gb) }
    // }
    //
    // pub fn set_execution_callback(&mut self) {
    //     unsafe { GB_set_execution_callback(self.inner.gb) }
    // }
    //
    // pub fn set_log_callback(&mut self) {
    //     unsafe { GB_set_log_callback(self.inner.gb) }
    // }
    //
    // pub fn cpu_disassemble(&mut self) {
    //     unsafe { GB_cpu_disassemble(self.inner.gb) }
    // }
    //
    // pub fn debugger_break(&mut self) {
    //     unsafe { GB_debugger_break(self.inner.gb) }
    // }
    //
    // pub fn debugger_clear_symbols(&mut self) {
    //     unsafe { GB_debugger_clear_symbols(self.inner.gb) }
    // }
    //
    // pub fn debugger_complete_substring(&mut self) {
    //     unsafe { GB_debugger_complete_substring(self.inner.gb) }
    // }
    //
    // pub fn debugger_describe_address(&mut self) {
    //     unsafe { GB_debugger_describe_address(self.inner.gb) }
    // }
    //
    // pub fn debugger_evaluate(&mut self) {
    //     unsafe { GB_debugger_evaluate(self.inner.gb) }
    // }
    //
    // pub fn debugger_execute_command(&mut self) {
    //     unsafe { GB_debugger_execute_command(self.inner.gb) }
    // }
    //
    // pub fn debugger_is_stopped(&mut self) {
    //     unsafe { GB_debugger_is_stopped(self.inner.gb) }
    // }
    //
    // pub fn debugger_load_symbol_file(&mut self) {
    //     unsafe { GB_debugger_load_symbol_file(self.inner.gb) }
    // }
    //
    // pub fn debugger_name_for_address(&mut self) {
    //     unsafe { GB_debugger_name_for_address(self.inner.gb) }
    // }
    //
    // pub fn debugger_set_disabled(&mut self) {
    //     unsafe { GB_debugger_set_disabled(self.inner.gb) }
    // }
    //
    // pub fn set_input_callback(&mut self) {
    //     unsafe { GB_set_input_callback(self.inner.gb) } // note that this is for the debugger, not the joypad
    // }
    //
    // pub fn set_async_input_callback(&mut self) {
    //     unsafe { GB_set_async_input_callback(self.inner.gb) } // note that this is for the debugger, not the joypad
    // }

    #[inline(always)]
    fn assert_not_running(&self) {
        self.inner.assert_not_running();
    }
}

struct GameboyStateInner {
    gb: *mut GB_gameboy_t,
    pixel_buffer: Vec<u32>,
    rendering_disabled: bool,
    user_data: Option<Box<dyn Any>>,
    rgb_encoding: RgbEncoding,
    events: Vec<Event>,
    enabled_events: EnabledEvents,
    read_memory_callback: Option<ReadMemoryCallback>,
    write_memory_callback: Option<WriteMemoryCallback>,
    running: bool,
    _phantom_pinned: PhantomPinned,
}

impl GameboyStateInner {
    fn reset_pixel_buffer(&mut self) {
        self.pixel_buffer.clear();

        if self.rendering_disabled {
            unsafe { GB_set_pixels_output(self.gb, core::ptr::null_mut()); }
        }
        else {
            let required_width = self.get_screen_width() * self.get_screen_height();
            self.pixel_buffer.resize(required_width, 0);
            unsafe { GB_set_pixels_output(self.gb, self.pixel_buffer.as_mut_ptr()); }
        }
    }

    fn get_screen_width(&self) -> usize {
        unsafe { GB_get_screen_width(self.gb) as usize }
    }

    fn get_screen_height(&self) -> usize {
        unsafe { GB_get_screen_height(self.gb) as usize }
    }

    fn get_user_data(&mut self) -> Option<&mut dyn Any> {
        self.user_data.as_mut().map(|m| m.as_mut())
    }

    #[inline(always)]
    fn is_running(&self) -> bool {
        self.running
    }

    #[inline(always)]
    fn assert_not_running(&self) {
        assert!(!self.is_running(), "A disallowed method was called while the Gameboy instance was running.");
    }
}

impl Drop for GameboyStateInner {
    fn drop(&mut self) {
        self.assert_not_running();
        unsafe {
            GB_dealloc(self.gb)
        }
    }
}

unsafe impl Send for GameboyStateInner {}
unsafe impl Sync for GameboyStateInner {}
