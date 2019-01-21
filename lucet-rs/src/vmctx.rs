use crate::errors::TerminationDetails;
use lucet_sys::*;
use std::os::raw::c_void;
use std::slice;

pub struct Vmctx {
    vmctx: *const lucet_vmctx,
}

impl Vmctx {
    pub fn get_heap(&self) -> &mut [u8] {
        unsafe {
            let heap_start = lucet_vmctx_get_heap(self.vmctx) as *mut u8;

            let heap_len = lucet_vmctx_current_memory(self.vmctx) as usize * 64 * 1024;
            slice::from_raw_parts_mut(heap_start, heap_len)
        }
    }
    pub fn get_delegate(&self) -> *const c_void {
        unsafe { lucet_vmctx_get_delegate(self.vmctx) as *const c_void }
    }
    pub fn terminate(&self, term_details: TerminationDetails) -> ! {
        unsafe {
            lucet_vmctx_terminate(self.vmctx, term_details.details);
            panic!("lucet vmctx terminate should never return")
        }
    }
    pub fn current_memory(&self) -> u32 {
        unsafe { lucet_vmctx_current_memory(self.vmctx) }
    }
    pub fn grow_memory(&self, additional_pages: u32) -> i32 {
        unsafe { lucet_vmctx_grow_memory(self.vmctx, additional_pages) }
    }
    pub fn get_globals(&self) -> *mut i64 {
        unsafe { lucet_vmctx_get_globals(self.vmctx) }
    }
    pub fn get_func_from_id(&self, table_id: u32, func_id: u32) -> Option<*mut c_void> {
        let func = unsafe { lucet_vmctx_get_func_from_id(self.vmctx, table_id, func_id) };
        if func.is_null() {
            None
        } else {
            Some(func)
        }
    }
    pub fn raw(&self) -> *const lucet_vmctx {
        self.vmctx
    }
}

impl From<*const lucet_vmctx> for Vmctx {
    fn from(vmctx: *const lucet_vmctx) -> Vmctx {
        Vmctx { vmctx }
    }
}
