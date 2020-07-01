mod borrow;

pub use lucet_wiggle_generate::bindings;
pub use lucet_wiggle_macro::from_witx;
pub use wiggle::*;

pub mod generate {
    pub use lucet_wiggle_generate::*;
}

pub mod runtime {
    use crate::borrow::BorrowChecker;
    use lucet_runtime::vmctx::Vmctx;
    use wiggle::{BorrowHandle, GuestError, GuestMemory, Region};

    pub struct LucetMemory<'a> {
        vmctx: &'a Vmctx,
        bc: BorrowChecker,
    }

    impl<'a> LucetMemory<'a> {
        pub fn new(vmctx: &'a Vmctx) -> LucetMemory {
            LucetMemory {
                vmctx,
                // Safety: we only construct a LucetMemory at the entry point of hostcalls, and
                // hostcalls are not re-entered, therefore there is exactly one BorrowChecker per
                // memory.
                bc: BorrowChecker::new(),
            }
        }
    }

    unsafe impl<'a> GuestMemory for LucetMemory<'a> {
        fn base(&self) -> (*mut u8, u32) {
            let mem = self.vmctx.heap_mut();
            let len = mem.len() as u32;
            let ptr = mem.as_ptr();
            (ptr as *mut u8, len)
        }
        fn has_outstanding_borrows(&self) -> bool {
            self.bc.has_outstanding_borrows()
        }
        fn is_borrowed(&self, r: Region) -> bool {
            self.bc.is_borrowed(r)
        }
        fn borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
            self.bc.borrow(r)
        }
        fn unborrow(&self, h: BorrowHandle) {
            self.bc.unborrow(h)
        }
    }
}
