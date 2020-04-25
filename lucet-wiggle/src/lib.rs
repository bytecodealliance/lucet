pub use lucet_wiggle_generate::bindings;
pub use lucet_wiggle_macro::from_witx;
pub use wiggle::{
    witx, GuestBorrows, GuestError, GuestErrorType, GuestMemory, GuestPtr, GuestType,
    GuestTypeTransparent, Pointee,
};

pub mod generate {
    pub use lucet_wiggle_generate::*;
}

pub mod runtime {
    use lucet_runtime::vmctx::Vmctx;
    use std::cell::RefMut;
    use wiggle::GuestMemory;

    pub struct LucetMemory<'a> {
        mem: RefMut<'a, [u8]>,
    }

    impl<'a> LucetMemory<'a> {
        pub fn new(vmctx: &Vmctx) -> LucetMemory {
            LucetMemory {
                mem: vmctx.heap_mut(),
            }
        }
    }

    unsafe impl<'a> GuestMemory for LucetMemory<'a> {
        fn base(&self) -> (*mut u8, u32) {
            let len = self.mem.len() as u32;
            let ptr = self.mem.as_ptr();
            (ptr as *mut u8, len)
        }
    }
}
