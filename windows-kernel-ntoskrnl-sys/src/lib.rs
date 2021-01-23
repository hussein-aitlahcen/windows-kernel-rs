#![no_std]

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![feature(untagged_unions)]

use windows_kernel_common_sys::*;

pub fn IoGetCurrentIrpStackLocation(pirp: PIRP) -> PIO_STACK_LOCATION {
    unsafe {
        return (&mut *pirp)
            .Tail
            .Overlay
            .__bindgen_anon_2
            .__bindgen_anon_1
            .CurrentStackLocation;
    }
}

pub fn IoGetNextIrpStackLocation(irp: PIRP) -> PIO_STACK_LOCATION {
    unsafe { IoGetCurrentIrpStackLocation(irp).offset(-1) }
}

pub fn IoSetCompletionRoutine(
    irp: PIRP,
    completion_routine: PIO_COMPLETION_ROUTINE,
    context: PVOID,
    invoke_on_success: bool,
    invoke_on_error: bool,
    invoke_on_cancel: bool,
) {
    let mut irp_sp = IoGetNextIrpStackLocation(irp);
    unsafe {
        (*irp_sp).CompletionRoutine = completion_routine;
        (*irp_sp).Context = context;
        (*irp_sp).Control = 0;
        if invoke_on_success {
            (*irp_sp).Control = SL_INVOKE_ON_SUCCESS as u8;
        }
        if invoke_on_error {
            (*irp_sp).Control |= SL_INVOKE_ON_ERROR as u8;
        }
        if invoke_on_cancel {
            (*irp_sp).Control |= SL_INVOKE_ON_CANCEL as u8;
        }
    }
}

#[link(name = "exception_free")]
extern "C" {
    pub fn SafeMmProbeAndLockPages(
        memory_descriptor_list: *mut MDL,
        access_mode: KPROCESSOR_MODE,
        operation: LOCK_OPERATION,
    ) -> NTSTATUS;
}

include!(concat!(env!("OUT_DIR"), "/bindings_ntifs.rs"));