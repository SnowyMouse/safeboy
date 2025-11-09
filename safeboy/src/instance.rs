use crate::Model;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::pin::Pin;

mod instance_inner;

pub use instance_inner::*;
use sameboy_sys::{GB_get_state_model_from_buffer, GB_model_t};

/// Safe wrapper around an emulator instance.
pub struct Gameboy {
    inner: Pin<Box<RunningGameboy>>
}

impl Gameboy {
    /// Instantiate a new instance.
    pub fn new(model: Model) -> Gameboy {
        let mut instance = Gameboy {
            inner: Box::pin(RunningGameboy::new(model))
        };
        instance.do_with_inner_mut(|inner| inner.finish_init());
        instance
    }

    fn do_with_inner_mut<T, F: FnOnce(&mut RunningGameboy) -> T>(&mut self, function: F) -> T {
        // SAFETY: We aren't moving or invalidating anything here.
        unsafe {
            let inner_mut = self.inner.as_mut().get_unchecked_mut();
            function(inner_mut)
        }
    }

    /// Set (or remove) the callbacks object.
    pub fn set_callbacks(&mut self, callbacks: Option<Box<dyn GameboyCallbacks>>) {
        self.do_with_inner_mut(|inner| inner.set_callbacks(callbacks))
    }

    /// Run for the smallest (atomic) unit of time for the emulator instance.
    ///
    /// Returns the number of 8 MiHz ticks (`8 388 608 Hz`) emulated.
    ///
    /// # Remarks
    ///
    /// If turbo is disabled, this may sleep to maintain the correct frame rate.
    pub fn run(&mut self) -> u32 {
        self.do_with_inner_mut(|inner| inner.run())
    }

    /// Runs until vblank is called.
    ///
    /// Returns the number of nanoseconds passed since the last frame.
    ///
    /// # Remarks
    ///
    /// This will not run at a capped speed. It is essentially the same as turning on turbo mode for
    /// one frame.
    pub fn run_frame(&mut self) -> u64 {
        self.do_with_inner_mut(|inner| inner.run_frame())
    }

    /// Set the model to `model` and reset the emulator.
    pub fn switch_model_and_reset(&mut self, model: Model) {
        self.do_with_inner_mut(|inner| inner.switch_model_and_reset(model));
    }

    /// Hard reset the emulator.
    pub fn reset(&mut self) {
        self.do_with_inner_mut(|inner| inner.reset());
    }

    /// Reset the emulator, but retain HRAM, tile data, object memory, palette data, and DMA state.
    pub fn partial_reset(&mut self) {
        self.do_with_inner_mut(|inner| inner.partial_reset());
    }

    /// Create a save state.
    pub fn create_save_state(&self) -> Vec<u8> {
        self.inner.create_save_state()
    }

    /// Load a save state.
    ///
    /// Returns `Err` if this function fails.
    pub fn load_save_state(&mut self, state: &[u8]) -> Result<(), ReadSaveStateError> {
        self.do_with_inner_mut(|inner| inner.load_save_state(state))
    }

    /// Rewind the emulator one frame backwards.
    ///
    /// Returns false if the end of the buffer was reached.
    pub fn rewind_pop(&mut self) -> bool {
        self.do_with_inner_mut(|inner| inner.rewind_pop())
    }
}

/// Read the save state and get its model.
pub fn model_for_save_state(save_state: &[u8]) -> Result<Model, ReadSaveStateError> {
    let mut model = 0;
    let result = unsafe { GB_get_state_model_from_buffer(save_state.as_ptr(), save_state.len(), &mut model) };
    match result {
        0 => Model::try_from(model as GB_model_t).map_err(|_| ReadSaveStateError::Other),
        _ => Err(ReadSaveStateError::Other)
    }
}

unsafe impl Send for Gameboy {}
unsafe impl Sync for Gameboy {}

/// Describes an error from loading a save state.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ReadSaveStateError {
    /// Unknown reason.
    Other
}

struct NullCallbacks;
impl GameboyCallbacks for NullCallbacks {}
