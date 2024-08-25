use std::any::Any;
use std::ffi::{c_char, c_uint, CStr};
use std::marker::PhantomPinned;
use std::panic::UnwindSafe;
use std::process::abort;
use sameboy_sys::*;
use crate::types::*;

pub struct Gameboy {
    inner: Box<GameboyStateInner>
}

impl Gameboy {
    pub fn new(model: Model) -> Self {
        unsafe {
            let gb = GB_alloc();
            let mut inner = Box::new(GameboyStateInner {
                gb,
                pixel_buffer: Vec::new(),
                rendering_disabled: false,
                rgb_encode_callback: default_rgb_encode_callback,
                apu_sample_callback: default_apu_sample_callback,
                rumble_callback: default_rumble_callback,
                vblank_callback: default_vblank_callback,
                read_memory_callback: default_read_memory_callback,
                write_memory_callback: default_write_memory_callback,
                pages: Vec::new(),
                user_data: None,
                _phantom_pinned: Default::default()
            });

            GB_init(gb, model as GB_model_t);
            GB_set_rendering_disabled(gb, true);
            GB_set_user_data(gb, inner.as_mut() as *mut GameboyStateInner as *mut _);

            // set dummy callbacks in case the user forgets to specify any
            GB_set_rgb_encode_callback(gb, Some(GameboyStateInner::rgb_encode_callback));
            GB_apu_set_sample_callback(gb, Some(GameboyStateInner::apu_sample_callback));
            GB_set_rumble_callback(gb, Some(GameboyStateInner::rumble_callback));
            GB_set_vblank_callback(gb, Some(GameboyStateInner::vblank_callback));

            Self { inner }
        }
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

    /// Sets the data which will be passed to all callbacks
    pub fn set_user_data(&mut self, data: Option<Box<dyn Any>>) {
        self.inner.user_data = data
    }

    /// Get the user data, if any
    pub fn get_user_data(&mut self) -> Option<&mut dyn Any> {
        self.inner.user_data.as_mut().map(|b| b.as_mut())
    }

    /// Set the model to `model` and reset the emulator.
    pub fn switch_model_and_reset(&mut self, model: Model) {
        unsafe { GB_switch_model_and_reset(self.inner.gb, model as GB_model_t) };
        self.inner.reset_pixel_buffer();
    }

    /// Hard reset the emulator.
    pub fn reset(&mut self) {
        unsafe { GB_reset(self.inner.gb) }
    }

    /// Reset the emulator, but retain HRAM, tile data, object memory, palette data, and DMA state.
    pub fn quick_reset(&mut self) {
        unsafe { GB_quick_reset(self.inner.gb) }
    }

    pub fn camera_updated(&mut self) {
        unsafe { GB_camera_updated(self.inner.gb) }
    }

    pub fn clear_joyp_accessed(&mut self) {
        unsafe { GB_clear_joyp_accessed(self.inner.gb) }
    }

    /// Connect an emulated printer.
    ///
    /// You can get all pages with [get_pages](Self::get_pages).
    ///
    /// Disable with [disconnect_serial](Self::disconnect_serial).
    pub fn connect_printer(&mut self) {
        unsafe { GB_connect_printer(self.inner.gb, Some(GameboyStateInner::printer_callback), Some(GameboyStateInner::printer_done_callback)) }
    }

    /// Get all pages, emptying the queue.
    pub fn get_pages(&mut self) -> Vec<PrinterPage> {
        let mut result = Vec::with_capacity(self.inner.pages.len());
        result.append(&mut self.inner.pages);
        result
    }

    // pub fn connect_workboy(&mut self) {
    //     unsafe { GB_connect_workboy(self.inner.gb) }
    // }

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

    pub fn gbs_switch_track(&mut self, track: u8) {
        unsafe { GB_gbs_switch_track(self.inner.gb, track) }
    }

    // pub fn get_apu_wave_table(&mut self) {
    //     unsafe { GB_get_apu_wave_table(self.inner.gb) }
    // }

    pub fn get_built_in_accessory(&self) -> Accessory {
        match unsafe { GB_get_built_in_accessory(self.inner.gb) } {
            n if n == GB_accessory_t_GB_ACCESSORY_WORKBOY => Accessory::Workboy,
            n if n == GB_accessory_t_GB_ACCESSORY_PRINTER => Accessory::Printer,
            n if n == GB_accessory_t_GB_ACCESSORY_NONE => Accessory::None,
            _ => unreachable!()
        }
    }

    pub fn get_channel_amplitude(&self, channel: AudioChannel) -> u8 {
        unsafe { GB_get_channel_amplitude(self.inner.gb, channel as GB_channel_t) }
    }

    pub fn get_channel_edge_triggered(&self, channel: AudioChannel) -> bool {
        unsafe { GB_get_channel_edge_triggered(self.inner.gb, channel as GB_channel_t) }
    }

    pub fn get_channel_period(&mut self, channel: AudioChannel) -> u16 {
        unsafe { GB_get_channel_period(self.inner.gb, channel as GB_channel_t) }
    }

    pub fn get_channel_volume(&mut self, channel: AudioChannel) -> u8 {
        unsafe { GB_get_channel_volume(self.inner.gb, channel as GB_channel_t) }
    }

    pub fn is_channel_muted(&self, channel: AudioChannel) -> bool {
        unsafe { GB_is_channel_muted(self.inner.gb, channel as GB_channel_t) }
    }

    pub fn set_channel_muted(&mut self, channel: AudioChannel, muted: bool) {
        unsafe { GB_set_channel_muted(self.inner.gb, channel as GB_channel_t, muted) }
    }

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
            (unsafe { std::slice::from_raw_parts_mut(access as *mut u8, size) }, bank)
        }
    }

    pub fn get_joyp_accessed(&self) -> bool {
        unsafe { GB_get_joyp_accessed(self.inner.gb) }
    }

    pub fn get_model(&self) -> Model {
        let model = unsafe { GB_get_model(self.inner.gb) } as u32;
        match model {
            n if n == Model::DMGB as u32 => Model::DMGB,
            n if n == Model::SGBNTSC as u32 => Model::SGBNTSC,
            n if n == Model::SGBPAL as u32 => Model::SGBPAL,
            n if n == Model::SGBNTSCNoSFC as u32 => Model::SGBNTSCNoSFC,
            n if n == Model::SGBPALNoSFC as u32 => Model::SGBPALNoSFC,
            n if n == Model::SGB2 as u32 => Model::SGB2,
            n if n == Model::SGB2NoSFC as u32 => Model::SGB2NoSFC,
            n if n == Model::MGB as u32 => Model::MGB,
            n if n == Model::CGB0 as u32 => Model::CGB0,
            n if n == Model::CGBA as u32 => Model::CGBA,
            n if n == Model::CGBB as u32 => Model::CGBB,
            n if n == Model::CGBC as u32 => Model::CGBC,
            n if n == Model::CGBD as u32 => Model::CGBD,
            n if n == Model::CGBE as u32 => Model::CGBE,
            n if n == Model::AGBA as u32 => Model::AGBA,
            n if n == Model::GBPA as u32 => Model::GBPA,
            _ => unreachable!("unknown model {:08X}", model)
        }
    }

    // pub fn get_oam_info(&mut self) {
    //     unsafe { GB_get_oam_info(self.inner.gb) }
    // }

    // pub fn get_palette(&mut self) {
    //     unsafe { GB_get_palette(self.inner.gb) }
    // }

    pub fn get_player_count(&mut self) -> u32 {
        unsafe { GB_get_player_count(self.inner.gb) }
    }

    pub fn get_registers(&self) -> Registers {
        unsafe {
            let registers = GB_get_registers(self.inner.gb);
            (*registers).into()
        }
    }

    pub fn get_rom_crc32(&mut self) -> u32 {
        unsafe { GB_get_rom_crc32(self.inner.gb) }
    }

    pub fn get_rom_title(&mut self) -> String {
        let mut title = [0u8; 17];
        unsafe { GB_get_rom_title(self.inner.gb, &mut title as *mut u8 as *mut c_char) };

        let c_str = CStr::from_bytes_until_nul(&title)
            .expect("should be null terminated or else SameBoy messed up...");

        c_str.to_string_lossy().to_string()
    }

    pub fn get_sample_rate(&self) -> u32 {
        unsafe { GB_get_sample_rate(self.inner.gb) }
    }

    pub fn get_screen_height(&self) -> usize {
        self.inner.get_screen_height()
    }

    pub fn get_screen_width(&self) -> usize {
        self.inner.get_screen_width()
    }

    pub fn get_unmultiplied_clock_rate(&self) -> u32 {
        unsafe { GB_get_unmultiplied_clock_rate(self.inner.gb) }
    }

    pub fn get_usual_frame_rate(&self) -> f64 {
        unsafe { GB_get_usual_frame_rate(self.inner.gb) }
    }

    pub fn has_accelerometer(&self) -> bool {
        unsafe { GB_has_accelerometer(self.inner.gb) }
    }

    pub fn icd_set_joyp(&mut self, value: u8) {
        unsafe { GB_icd_set_joyp(self.inner.gb, value) }
    }

    pub fn is_background_rendering_disabled(&self) -> bool {
        unsafe { GB_is_background_rendering_disabled(self.inner.gb) }
    }

    pub fn is_cgb(&self) -> bool {
        unsafe { GB_is_cgb(self.inner.gb) }
    }

    pub fn is_cgb_in_cgb_mode(&self) -> bool {
        unsafe { GB_is_cgb_in_cgb_mode(self.inner.gb) }
    }

    pub fn is_hle_sgb(&self) -> bool {
        unsafe { GB_is_hle_sgb(self.inner.gb) }
    }

    pub fn is_object_rendering_disabled(&self) -> bool {
        unsafe { GB_is_object_rendering_disabled(self.inner.gb) }
    }

    pub fn is_odd_frame(&self) -> bool {
        unsafe { GB_is_odd_frame(self.inner.gb) }
    }

    pub fn is_sgb(&self) -> bool {
        unsafe { GB_is_sgb(self.inner.gb) }
    }

    pub fn load_sram_from_buffer(&mut self, buffer: &[u8]) {
        unsafe { GB_load_battery_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) }
    }

    pub fn load_boot_rom_from_buffer(&mut self, buffer: &[u8]) {
        unsafe { GB_load_boot_rom_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) }
    }

    // pub fn load_gbs_from_buffer(&mut self) {
    //     unsafe { GB_load_gbs_from_buffer(self.inner.gb) }
    // }

    pub fn load_rom_from_buffer(&mut self, buffer: &[u8]) {
        unsafe { GB_load_rom_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) }
    }

    pub fn load_state_from_buffer(&mut self, buffer: &[u8]) -> Result<(), ()> {
        let result = unsafe { GB_load_state_from_buffer(self.inner.gb, buffer.as_ptr(), buffer.len()) };
        if result != 0 {
            Err(())
        }
        else {
            Ok(())
        }
    }

    pub fn read_memory(&mut self, addr: u16) -> u8 {
        unsafe { GB_read_memory(self.inner.gb, addr) }
    }

    pub fn rewind_pop(&mut self) -> bool {
        unsafe { GB_rewind_pop(self.inner.gb) }
    }

    pub fn rewind_reset(&mut self) {
        unsafe { GB_rewind_reset(self.inner.gb) }
    }

    pub fn rom_supports_alarms(&mut self) -> bool {
        unsafe { GB_rom_supports_alarms(self.inner.gb) }
    }

    pub fn time_to_alarm(&mut self) -> u32 {
        unsafe { GB_time_to_alarm(self.inner.gb) as u32 }
    }

    /// Run for a few cycles.
    ///
    /// Returns the number of 8 MiHz cycles that passed.
    pub fn run(&mut self) -> u64 {
        unsafe { GB_run(self.inner.gb) as u64 }
    }

    /// Run for one frame.
    ///
    /// Returns the number of nanoseconds passed since the last frame.
    pub fn run_frame(&mut self) -> u64 {
        unsafe { GB_run_frame(self.inner.gb) }
    }

    /// Read memory at the address.
    ///
    /// NOTE: This will still trigger the read_memory_callback.
    pub fn safe_read_memory(&mut self, addr: u16) -> u8 {
        unsafe { GB_safe_read_memory(self.inner.gb, addr) }
    }

    pub fn get_sram_size(&self) -> usize {
        (unsafe { GB_save_battery_size(self.inner.gb) }) as usize
    }

    pub fn read_sram_to_buffer(&self, data: &mut [u8]) {
        assert_eq!(data.len(), self.get_sram_size());
        unsafe {
            GB_save_battery_to_buffer(self.inner.gb, data.as_mut_ptr(), data.len());
        }
    }

    pub fn read_sram_to_vec(&self) -> Vec<u8> {
        let len = self.get_sram_size();
        let mut data = Vec::with_capacity(len);
        unsafe {
            data.set_len(len);
        }
        self.read_sram_to_buffer(&mut data);
        data
    }

    pub fn get_save_state_size(&self) -> usize {
        unsafe { GB_get_save_state_size(self.inner.gb) }
    }

    pub fn read_save_state_to_buffer(&self, data: &mut [u8]) {
        assert_eq!(data.len(), self.get_save_state_size());
        unsafe {
            GB_save_state_to_buffer(self.inner.gb, data.as_mut_ptr());
        }
    }

    pub fn read_save_state_to_vec(&self) -> Vec<u8> {
        let len = self.get_save_state_size();
        let mut data = Vec::with_capacity(len);
        unsafe {
            data.set_len(len);
        }
        self.read_save_state_to_buffer(&mut data);
        data
    }

    pub fn set_apu_sample_callback(&mut self, callback: fn(callback: Option<&mut dyn Any>, left: i16, right: i16)) {
        self.inner.apu_sample_callback = callback;
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
    //     unsafe { GB_add_cheat(self.inner.gb) }
    // }
    //
    // pub fn update_cheat(&mut self) {
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

    pub fn set_clock_multiplier(&mut self, multiplier: f64) {
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
            colors: std::array::from_fn(|f| palette[f].into())
        };
        unsafe { GB_set_palette(self.inner.gb, &palette) }
    }

    pub fn set_rendering_disabled(&mut self, disabled: bool) {
        unsafe { GB_set_rendering_disabled(self.inner.gb, disabled) };
        self.inner.rendering_disabled = disabled;
        self.inner.reset_pixel_buffer();
    }

    pub fn get_pixel_buffer(&self) -> &[u32] {
        self.inner.pixel_buffer.as_slice()
    }

    pub fn set_rewind_length(&mut self, seconds: f64) {
        unsafe { GB_set_rewind_length(self.inner.gb, seconds) }
    }

    pub fn set_rtc_mode(&mut self, mode: RTCMode) {
        unsafe { GB_set_rtc_mode(self.inner.gb, mode as GB_rtc_mode_t) }
    }

    pub fn set_rtc_multiplier(&mut self, multiplier: f64) {
        unsafe { GB_set_rtc_multiplier(self.inner.gb, multiplier) }
    }

    pub fn set_rumble_mode(&mut self, mode: Rumble) {
        unsafe { GB_set_rumble_mode(self.inner.gb, mode as GB_rumble_mode_t) }
    }

    pub fn set_rumble_callback(&mut self, callback: fn(callback: Option<&mut dyn Any>, rumble_amplitude: f64)) {
        self.inner.rumble_callback = callback
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        unsafe { GB_set_sample_rate(self.inner.gb, sample_rate as c_uint) }
    }

    pub fn set_sample_rate_by_clocks(&mut self, clocks_per_sample: f64) {
        unsafe { GB_set_sample_rate_by_clocks(self.inner.gb, clocks_per_sample) }
    }

    pub fn set_turbo_mode(&mut self, turbo: bool, no_frame_skip: bool) {
        unsafe { GB_set_turbo_mode(self.inner.gb, turbo, no_frame_skip) }
    }

    pub fn write_memory(&mut self, addr: u16, value: u8) {
        unsafe { GB_write_memory(self.inner.gb, addr, value) }
    }

    pub fn set_rgb_encode_callback(&mut self, callback: Option<fn(callback: Option<&mut dyn Any>, red: u8, green: u8, blue: u8) -> u32>) {
        self.inner.rgb_encode_callback = callback.unwrap_or(default_rgb_encode_callback);
    }

    pub fn set_vblank_callback(&mut self, callback: Option<fn(callback: Option<&mut dyn Any>, vblank_type: VBlankType)>) {
        self.inner.vblank_callback = callback.unwrap_or(default_vblank_callback);
    }

    pub fn set_read_memory_callback(&mut self, callback: Option<fn(callback: Option<&mut dyn Any>, addr: u16, data: u8) -> u8>) {
        if let Some(callback) = callback {
            self.inner.read_memory_callback = callback;
            unsafe { GB_set_read_memory_callback(self.inner.gb, Some(GameboyStateInner::read_memory_callback)); }
        }
        else {
            self.inner.read_memory_callback = default_read_memory_callback;
            unsafe { GB_set_read_memory_callback(self.inner.gb, None); }
        }
    }

    pub fn set_write_memory_callback(&mut self, callback: Option<fn(callback: Option<&mut dyn Any>, addr: u16, data: u8) -> bool>) {
        if let Some(callback) = callback {
            self.inner.write_memory_callback = callback;
            unsafe { GB_set_write_memory_callback(self.inner.gb, Some(GameboyStateInner::write_memory_callback)); }
        }
        else {
            self.inner.write_memory_callback = default_write_memory_callback;
            unsafe { GB_set_write_memory_callback(self.inner.gb, None); }
        }
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
}

struct GameboyStateInner {
    gb: *mut GB_gameboy_t,
    pixel_buffer: Vec<u32>,
    rendering_disabled: bool,
    rgb_encode_callback: fn(user_data: Option<&mut dyn Any>, red: u8, green: u8, blue: u8) -> u32,
    apu_sample_callback: fn(user_data: Option<&mut dyn Any>, left: i16, right: i16),
    vblank_callback: fn(user_data: Option<&mut dyn Any>, vblank_type: VBlankType),
    rumble_callback: fn(user_data: Option<&mut dyn Any>, rumble_amplitude: f64),
    read_memory_callback: fn(user_data: Option<&mut dyn Any>, addr: u16, data: u8) -> u8,
    write_memory_callback: fn(user_data: Option<&mut dyn Any>, addr: u16, data: u8) -> bool,
    pages: Vec<PrinterPage>,
    user_data: Option<Box<dyn Any>>,
    _phantom_pinned: PhantomPinned,
}

impl GameboyStateInner {
    fn reset_pixel_buffer(&mut self) {
        self.pixel_buffer.clear();

        if self.rendering_disabled {
            unsafe { GB_set_pixels_output(self.gb, std::ptr::null_mut()); }
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

    extern "C" fn read_memory_callback(gb: *mut GB_gameboy_t, addr: u16, data: u8) -> u8 {
        Self::catch_panic_and_die("read", || {
            let this = Self::resolve_self(gb);
            (this.read_memory_callback)(this.get_user_data(), addr, data)
        })
    }

    extern "C" fn write_memory_callback(gb: *mut GB_gameboy_t, addr: u16, data: u8) -> bool {
        Self::catch_panic_and_die("read", || {
            let this = Self::resolve_self(gb);
            (this.write_memory_callback)(this.get_user_data(), addr, data)
        })
    }

    extern "C" fn vblank_callback(gb: *mut GB_gameboy_t, vblank_type: GB_vblank_type_t) {
        Self::catch_panic_and_die("vblank", || {
            let this = Self::resolve_self(gb);
            (this.vblank_callback)(this.get_user_data(), vblank_type.into())
        })
    }

    extern "C" fn rumble_callback(gb: *mut GB_gameboy_t, rumble_amplitude: f64) {
        Self::catch_panic_and_die("rumble", || {
            let this = Self::resolve_self(gb);
            (this.rumble_callback)(this.get_user_data(), rumble_amplitude)
        })
    }

    extern "C" fn apu_sample_callback(gb: *mut GB_gameboy_t, sample: *mut GB_sample_t) {
        Self::catch_panic_and_die("apu_sample", || {
            let this = Self::resolve_self(gb);
            let sample = unsafe { *sample };
            (this.apu_sample_callback)(this.get_user_data(), sample.left, sample.right)
        })
    }

    extern "C" fn rgb_encode_callback(gb: *mut GB_gameboy_t, r: u8, g: u8, b: u8) -> u32 {
        Self::catch_panic_and_die("rgb_encode", || {
            let this = Self::resolve_self(gb);
            (this.rgb_encode_callback)(this.get_user_data(), r, g, b)
        })
    }

    fn get_user_data(&mut self) -> Option<&mut dyn Any> {
        self.user_data.as_mut().map(|m| m.as_mut())
    }

    /// Catch an unwinding panic, and abort if this occurs.
    ///
    /// We need to catch unwinds inside of callbacks, since it is undefined behavior to unwind
    /// across FFI bounds.
    ///
    /// The callback could be anything, after all!
    fn catch_panic_and_die<T, F: FnOnce() -> T + UnwindSafe>(name: &'static str, action: F) -> T {
        std::panic::catch_unwind(action).unwrap_or_else(|_| { eprintln!("{name} callback panicked"); abort() })
    }

    extern "C" fn printer_callback(
        gb: *mut GB_gameboy_t,
        image: *mut u32,
        height: u8,
        top_margin: u8,
        bottom_margin: u8,
        exposure: u8
    ) {
        let this = Self::resolve_self(gb);

        let height = height as usize;
        let top_margin = top_margin as usize;
        let bottom_margin = bottom_margin as usize;
        let total_height = height + top_margin + bottom_margin;
        let width = 160;

        let mut data = vec![0xFFFFFFFF; total_height * width];
        let input_data = unsafe { std::slice::from_raw_parts_mut(image, height * width) };
        let output_start = width * top_margin;
        data[output_start..output_start + height * width].copy_from_slice(input_data);

        let page = PrinterPage {
            width,
            top_margin,
            data,
            height,
            bottom_margin,
            exposure
        };

        this.pages.push(page);
    }

    extern "C" fn printer_done_callback(_: *mut GB_gameboy_t) {}

    fn resolve_self(gb: *mut GB_gameboy_t) -> &'static mut Self {
        // SAFETY: Should be OK as we set the user data to this instance, and we use a pin.
        unsafe { (GB_get_user_data(gb) as *mut Self).as_mut().unwrap() }
    }
}

impl Drop for Gameboy {
    fn drop(&mut self) {
        unsafe {
            GB_dealloc(self.inner.gb)
        }
    }
}


fn default_rgb_encode_callback(_: Option<&mut dyn Any>, r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | (( r as u32 ) << 16) | (( g as u32 ) << 8) | ( b as u32 )
}

fn default_apu_sample_callback(_: Option<&mut dyn Any>, _: i16, _: i16) {}

fn default_rumble_callback(_: Option<&mut dyn Any>, _: f64) {}

fn default_vblank_callback(_: Option<&mut dyn Any>, _: VBlankType) {}

fn default_read_memory_callback(_: Option<&mut dyn Any>, _: u16, data: u8) -> u8 { data }

fn default_write_memory_callback(_: Option<&mut dyn Any>, _: u16, _: u8) -> bool { true }

unsafe impl Send for GameboyStateInner {}
unsafe impl Sync for GameboyStateInner {}
