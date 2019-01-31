use crate::alloc::instance_heap_offset;
use crate::context::Context;
use crate::instance::{
    Instance, InstanceInternal, State, CURRENT_INSTANCE, HOST_CTX, WASM_PAGE_SIZE,
};
use libc::c_void;
use std::sync::Once;

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

impl Instance {
    /// Get an Instance from the `vmctx` pointer.
    ///
    /// Only safe to call from within the guest context.
    unsafe fn from_vmctx<'a>(vmctx: *const c_void) -> &'a mut Instance {
        assert!(!vmctx.is_null(), "vmctx is not null");

        let inst_ptr = (vmctx as usize - instance_heap_offset()) as *mut Instance;

        // We shouldn't actually need to access the thread local, only the exception handler should
        // need to. But, as long as the thread local exists, we should make sure that the guest
        // hasn't pulled any shenanigans and passed a bad vmctx. (Codegen should ensure the guest
        // cant pull any shenanigans but there have been bugs before.)
        CURRENT_INSTANCE.with(|current_instance| {
            if let Some(current_inst_ptr) = current_instance.borrow().map(|nn| nn.as_ptr()) {
                assert!(
                    inst_ptr == current_inst_ptr,
                    "vmctx corresponds to current instance"
                );
            } else {
                panic!(
                    "current instance is not set; thread local storage failure can indicate \
                     dynamic linking issues"
                );
            }
        });

        let inst = inst_ptr.as_mut().unwrap();
        assert!(inst.valid_magic());
        inst
    }

    /// Terminate the guest and swap back to the host context.
    ///
    /// Only safe to call from within the guest context.
    unsafe fn terminate(&mut self, info: *mut c_void) -> ! {
        self.state = State::Terminated { info };
        HOST_CTX.with(|host_ctx| Context::set(&*host_ctx.get()))
    }
}

static VMCTX_CAPI_INIT: Once = Once::new();

/// Should never actually be called, but should be reachable via a trait method to prevent DCE.
pub fn vmctx_capi_init() {
    use std::ptr::read_volatile;
    VMCTX_CAPI_INIT.call_once(|| unsafe {
        read_volatile(lucet_vmctx_get_heap as *const extern "C" fn());
        read_volatile(lucet_vmctx_current_memory as *const extern "C" fn());
        read_volatile(lucet_vmctx_grow_memory as *const extern "C" fn());
        read_volatile(lucet_vmctx_check_heap as *const extern "C" fn());
        read_volatile(lucet_vmctx_terminate as *const extern "C" fn());
        read_volatile(lucet_vmctx_get_delegate as *const extern "C" fn());
    });
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_heap(vmctx: *mut c_void) -> *mut u8 {
    Vmctx::from_raw(vmctx).instance().alloc().slot().heap as *mut u8
}

/// Get the number of WebAssembly pages currently in the heap.
#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_current_memory(vmctx: *mut c_void) -> libc::uint32_t {
    Vmctx::from_raw(vmctx).instance().alloc().heap_len() as u32 / WASM_PAGE_SIZE
}

#[no_mangle]
/// Grows the guest heap by the given number of WebAssembly pages.
///
/// On success, returns the number of pages that existed before the call. On failure, returns `-1`.
pub unsafe extern "C" fn lucet_vmctx_grow_memory(
    vmctx: *const c_void,
    additional_pages: libc::uint32_t,
) -> libc::int32_t {
    let inst = Instance::from_vmctx(vmctx);
    if let Ok(old_pages) = inst.grow_memory(additional_pages) {
        old_pages as libc::int32_t
    } else {
        -1
    }
}

#[no_mangle]
/// Check if a memory region is inside the instance heap.
pub unsafe extern "C" fn lucet_vmctx_check_heap(
    vmctx: *const c_void,
    ptr: *mut c_void,
    len: libc::size_t,
) -> bool {
    let inst = Instance::from_vmctx(vmctx);
    inst.check_heap(ptr, len)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_terminate(vmctx: *const c_void, info: *mut c_void) {
    let inst = Instance::from_vmctx(vmctx);
    inst.terminate(info);
}

#[no_mangle]
/// Get the delegate object for the current instance.
pub unsafe extern "C" fn lucet_vmctx_get_delegate(vmctx: *const c_void) -> *mut c_void {
    let inst = Instance::from_vmctx(vmctx);
    inst.embed_ctx
}
