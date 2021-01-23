#![no_std]

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(untagged_unions)]

use windows_kernel_common_sys::*;

include!(concat!(env!("OUT_DIR"), "/bindings_wsk.rs"));
