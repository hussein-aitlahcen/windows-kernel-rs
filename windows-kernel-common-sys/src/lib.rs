#![no_std]

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(untagged_unions)]

pub use cty::*;
pub use winapi::shared::{ntdef::*, basetsd::*, ws2def::*, ntstatus};
pub use winapi::um::ntlsa::*;

include!(concat!(env!("OUT_DIR"), "/bindings_base.rs"));