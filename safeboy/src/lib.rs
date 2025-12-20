//! # safeboy
//! 
//! Safe Rust bindings for the SameBoy emulator, an accurate Game Boy Color emulator written in C
//! by Lior Halphon.

#![no_std]
#![warn(missing_docs)]

const _: () = {
    assert!(size_of::<usize>() >= size_of::<u32>());
};

pub use sameboy_sys::GB_VERSION;

extern crate alloc;

pub mod rgb_encoder;

mod instance;

pub use instance::*;

mod model;
pub use model::*;
