pub use lucet_wiggle_macro::lucet_integration;
pub use wiggle::*;

pub mod runtime {
    use wiggle::{borrow::BorrowChecker, BorrowHandle, GuestError, GuestMemory, Region};

    pub struct LucetMemory<'a> {
        memory: &'a mut [u8],
        bc: BorrowChecker,
    }

    impl<'a> LucetMemory<'a> {
        pub fn new(memory: &'a mut [u8]) -> LucetMemory {
            LucetMemory {
                memory,
                // Safety: we only construct a LucetMemory at the entry point of hostcalls, and
                // hostcalls are not re-entered, therefore there is exactly one BorrowChecker per
                // memory.
                bc: BorrowChecker::new(),
            }
        }
    }

    unsafe impl<'a> GuestMemory for LucetMemory<'a> {
        fn base(&self) -> (*mut u8, u32) {
            let len = self.memory.len() as u32;
            let ptr = self.memory.as_ptr();
            (ptr as *mut u8, len)
        }
        fn has_outstanding_borrows(&self) -> bool {
            self.bc.has_outstanding_borrows()
        }
        fn is_mut_borrowed(&self, r: Region) -> bool {
            self.bc.is_mut_borrowed(r)
        }
        fn is_shared_borrowed(&self, r: Region) -> bool {
            self.bc.is_shared_borrowed(r)
        }
        fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
            self.bc.mut_borrow(r)
        }
        fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
            self.bc.shared_borrow(r)
        }
        fn mut_unborrow(&self, h: BorrowHandle) {
            self.bc.mut_unborrow(h)
        }
        fn shared_unborrow(&self, h: BorrowHandle) {
            self.bc.shared_unborrow(h)
        }
    }
}
