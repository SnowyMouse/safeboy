//! Safe Rust bindings for the SameBoy emulator

pub mod types;
mod gb;

pub use gb::Gameboy;
pub use gb::event::Event;
