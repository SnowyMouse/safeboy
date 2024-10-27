use crate::types::{PrinterPage, VBlankType};

pub enum Event {
    Sample { left: i16, right: i16 },
    Rumble { amplitude: f64 },
    VBlank { vblank_type: VBlankType },
    PrintedPage { page: PrinterPage },
    MemoryRead { address: u16, original_data: u8, final_data: u8 },
    MemoryWrite { address: u16, data: u8, prevented_by_callback: bool },
}

pub(crate) mod inner {
    use sameboy_sys::{GB_gameboy_t, GB_get_user_data, GB_sample_t, GB_vblank_type_t};
    use crate::gb::{GameboyStateInner, RgbEncoding};
    use crate::gb::event::Event;
    use crate::types::PrinterPage;

    pub unsafe extern "C" fn rgb_encode_callback(gb: *mut GB_gameboy_t, r: u8, g: u8, b: u8) -> u32 {
        match get_instance(gb).rgb_encoding {
            RgbEncoding::R8G8B8X8 => 0xFF | ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8),
            RgbEncoding::B8G8R8X8 => 0xFF | ((b as u32) << 24) | ((g as u32) << 16) | ((r as u32) << 8),
            RgbEncoding::X8R8G8B8 => 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | ((b as u32) << 0),
            RgbEncoding::X8B8G8R8 => 0xFF000000 | ((b as u32) << 16) | ((g as u32) << 8) | ((r as u32) << 0)
        }
    }

    pub unsafe extern "C" fn sample_callback(gb: *mut GB_gameboy_t, sample: *mut GB_sample_t) {
        let instance = get_instance(gb);
        if instance.enabled_events.sample {
            get_instance(gb).events.push(Event::Sample { left: (*sample).left, right: (*sample).right })
        }
    }

    pub unsafe extern "C" fn rumble_callback(gb: *mut GB_gameboy_t, amplitude: f64) {
        let instance = get_instance(gb);
        if instance.enabled_events.rumble {
            instance.events.push(Event::Rumble { amplitude })
        }
    }

    pub unsafe extern "C" fn vblank_callback(gb: *mut GB_gameboy_t, vblank: GB_vblank_type_t) {
        let instance = get_instance(gb);
        if instance.enabled_events.vblank {
            instance.events.push(Event::VBlank { vblank_type: vblank.into() })
        }
    }

    pub unsafe extern "C" fn printer_callback(
        gb: *mut GB_gameboy_t,
        image: *mut u32,
        height: u8,
        top_margin: u8,
        bottom_margin: u8,
        exposure: u8
    ) {
        let instance = get_instance(gb);
        if !instance.enabled_events.printer {
            return
        }

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

        instance.events.push(Event::PrintedPage { page })
    }

    pub extern "C" fn printer_done_callback(_: *mut GB_gameboy_t) {}

    pub unsafe extern "C" fn read_memory_callback(gb: *mut GB_gameboy_t, address: u16, original_data: u8) -> u8 {
        let user_data = get_instance(gb).get_user_data();

        let final_data = (get_instance(gb).read_memory_callback)(user_data, address, original_data);

        if get_instance(gb).enabled_events.memory_read {
            get_instance(gb).events.push(Event::MemoryRead { address, original_data, final_data })
        }

        final_data
    }

    pub unsafe extern "C" fn write_memory_callback(gb: *mut GB_gameboy_t, address: u16, data: u8) -> bool {
        let user_data = get_instance(gb).get_user_data();

        let allow = (get_instance(gb).write_memory_callback)(user_data, address, data);

        if get_instance(gb).enabled_events.memory_write {
            get_instance(gb).events.push(Event::MemoryWrite { address, data, prevented_by_callback: !allow })
        }

        allow
    }


    #[inline(always)]
    unsafe fn get_instance(gb: *mut GB_gameboy_t) -> &'static mut GameboyStateInner {
        let userdata = GB_get_user_data(gb);
        (userdata as *mut GameboyStateInner)
            .as_mut()
            .expect("null pointer passed?")
    }
}
