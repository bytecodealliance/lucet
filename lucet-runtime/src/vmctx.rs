use crate::instance::Instance;
use libc::c_void;

pub struct Vmctx {
    vmctx: *mut c_void,
}

impl Vmctx {
    pub unsafe fn from_raw(vmctx: *mut c_void) -> Vmctx {
        assert!(!vmctx.is_null());
        Vmctx { vmctx }
    }

    fn instance(&self) -> &Instance {
        unsafe { Instance::from_vmctx(self.vmctx) }
    }

    fn instance_mut(&mut self) -> &mut Instance {
        unsafe { Instance::from_vmctx(self.vmctx) }
    }

    pub fn heap(&self) -> &[u8] {
        self.instance().heap()
    }

    pub fn heap_mut(&mut self) -> &[u8] {
        self.instance_mut().heap_mut()
    }

    pub fn check_heap(&self, ptr: *const c_void, len: usize) -> bool {
        self.instance().check_heap(ptr, len)
    }

    pub fn embed_ctx(&self) -> *mut c_void {
        self.instance().embed_ctx
    }

    pub fn terminate(&mut self, info: *mut c_void) -> ! {
        unsafe { self.instance_mut().terminate(info) }
    }
}
