#![no_std]

// we don't want to change anything with the naming
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

// can't really do anything about bindgen's output
#![allow(unnecessary_transmutes)]
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(not(feature = "bindgen"))]
include!("bindings_pregenerated.rs");

/// The SameBoy core's version
pub const GB_VERSION: &str = env!("GB_VERSION");
