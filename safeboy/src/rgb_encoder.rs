//! RGB encoder methods.

/// Encode the given color values into A8R8G8B8.
pub const fn encode_a8r8g8b8(r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | ((b as u32) << 0)
}

/// Encode the given color values into A8B8G8R8.
pub const fn encode_a8b8g8a8(r: u8, g: u8, b: u8) -> u32 {
    0xFF000000 | ((b as u32) << 16) | ((g as u32) << 8) | ((r as u32) << 0)
}

/// Encode the given color values into R8G8B8A8.
pub const fn encode_r8g8b8a8(r: u8, g: u8, b: u8) -> u32 {
    0x000000FF | ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8)
}

/// Encode the given color values into B8G8R8A8.
pub const fn encode_b8g8a8a8(r: u8, g: u8, b: u8) -> u32 {
    0x000000FF | ((b as u32) << 24) | ((g as u32) << 16) | ((r as u32) << 8)
}

/// Encodes RGB into a 32-bit packed int.
pub type RgbEncoder = fn(r: u8, g: u8, b: u8) -> u32;