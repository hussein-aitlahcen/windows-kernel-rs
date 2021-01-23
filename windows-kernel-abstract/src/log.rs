pub use windows_kernel_ntoskrnl_sys::DbgPrint;

#[macro_export]
macro_rules! log {
    ($string: expr) => {
        unsafe {
            $crate::log::DbgPrint(::core::mem::transmute(concat!("[>] ", $string, "\0").as_ptr()))
        }
    };

    ($string: expr, $($x:tt)*) => {
        unsafe {
            #[allow(unused_unsafe)]
            $crate::log::DbgPrint(::core::mem::transmute(concat!("[>] ", $string, "\0").as_ptr()), $($x)*)
        }
    };
}
