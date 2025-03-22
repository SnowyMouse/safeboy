//! Safe Rust bindings for the SameBoy emulator

#![no_std]
extern crate alloc;

pub mod types;
mod gb;

pub use gb::Gameboy;
pub use gb::event::Event;
