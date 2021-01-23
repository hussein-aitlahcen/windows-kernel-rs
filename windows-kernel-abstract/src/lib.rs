#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod log;
pub mod mdl;

use either::{Either, Left, Right};
use windows_kernel_common_sys::{
    ntstatus, NTSTATUS,
};

pub trait ErrorLike: Sized {
    fn as_either(&self) -> Either<Self, ()>;
}

impl ErrorLike for NTSTATUS {
    fn as_either(&self) -> Either<NTSTATUS, ()> {
        match *self {
            ntstatus::STATUS_SUCCESS => Right(()),
            err => Left(err),
        }
    }
}