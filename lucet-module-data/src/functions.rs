use crate::traps::{TrapManifest, TrapSite};

use std::slice::from_raw_parts;

// The layout of this struct is very tightly coupled to lucetc's `write_function_manifest`!
//
// Specifically, `write_function_manifest` sets up relocations on `code_addr` and `traps_addr`.
// It does not explicitly serialize a correctly formed `FunctionSpec`, because addresses
// for these fields do not exist until the object is loaded in the future.
//
// So `write_function_manifest` has implicit knowledge of the layout of this structure
// (including padding bytes between `code_len` and `traps_addr`)
#[repr(C)]
#[derive(Clone, Debug)]
pub struct FunctionSpec {
    code_addr: u64,
    code_len: u32,
    traps_addr: u64,
    traps_len: u64
}

impl FunctionSpec {
    pub fn new(code_addr: u64, code_len: u32, traps_addr: u64, traps_len: u64) -> Self {
        FunctionSpec { code_addr, code_len, traps_addr, traps_len }
    }
    pub fn code_len(&self) -> u32 {
        self.code_len
    }
    pub fn traps_len(&self) -> u64 {
        self.traps_len
    }
    pub fn contains(&self, addr: u64) -> bool {
        // TODO This *may* be an off by one - replicating the check in
        // looking up trap manifest addresses. Need to verify if the
        // length produced by Cranelift is of an inclusive or exclusive range
        addr >= self.code_addr && (addr - self.code_addr) <= (self.code_len as u64)
    }
    pub fn relative_addr(&self, addr: u64) -> Option<u32> {
        if let Some(offset) = addr.checked_sub(self.code_addr) {
            if offset < (self.code_len as u64) {
                // self.code_len is u32, so if the above check succeeded
                // offset must implicitly be <= u32::MAX - the following
                // conversion will not truncate bits
                return Some(offset as u32);
            }
        }

        None
    }
    pub fn traps(&self) -> Option<TrapManifest> {
        let traps_ptr = self.traps_addr as *const TrapSite;
        if !traps_ptr.is_null() {
            let traps_slice =
                unsafe {
                    from_raw_parts(traps_ptr, self.traps_len as usize)
                };
            Some(TrapManifest::new(traps_slice))
        } else {
            None
        }
    }
}
