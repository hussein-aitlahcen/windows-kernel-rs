use crate::ErrorLike;
use either::Either;
use windows_kernel_common_sys::{KPROCESSOR_MODE, LOCK_OPERATION, NTSTATUS, MDL};
use windows_kernel_ntoskrnl_sys::{MmUnlockPages, SafeMmProbeAndLockPages};

pub struct LockedMdl<'a> {
    mdl: &'a mut MDL,
}

impl<'a> LockedMdl<'a> {
    pub fn new(
        mdl: &'a mut MDL,
        access_mode: KPROCESSOR_MODE,
        operation: LOCK_OPERATION,
    ) -> Either<NTSTATUS, Self> {
        unsafe { SafeMmProbeAndLockPages(mdl, access_mode, operation) }
            .as_either()
            .map_right(move |_| LockedMdl { mdl })
    }
}

impl<'a> Drop for LockedMdl<'a> {
    fn drop(&mut self) {
        unsafe {
            MmUnlockPages(self.mdl);
        }
    }
}
