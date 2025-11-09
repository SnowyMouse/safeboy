#![no_std]

// we don't want to change anything with the naming
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

// can't really do anything about bindgen's output
#![allow(unnecessary_transmutes)]
#![allow(unsafe_op_in_unsafe_fn)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// SameBoy's version
pub const GB_VERSION: &str = env!("GB_VERSION");
