#[macro_export]
macro_rules! function_bytes_slice {
    ($fn_start:expr) => {
        function_bytes_slice!($fn_start, 1)
    };
    ($fn_start:expr, $fn_len:expr) => {
        unsafe {
            std::slice::from_raw_parts($fn_start as *const extern "C" fn() as *const u8, $fn_len)
        };
    };
}

pub mod build;
pub mod entrypoint;
pub mod globals;
pub mod guest_fault;
pub mod helpers;
pub mod host;
pub mod memory;
pub mod stack;
pub mod start;
pub mod strcmp;
