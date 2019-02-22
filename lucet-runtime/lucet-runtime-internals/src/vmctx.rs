//! Interfaces for accessing instance data from hostcalls.
//!
//! This module contains both a Rust-friendly API ([`Vmctx`](struct.Vmctx.html)) as well as C-style
//! exports for compatibility with hostcalls written against `lucet-runtime-c`.

use crate::alloc::instance_heap_offset;
use crate::context::Context;
use crate::error::Error;
use crate::instance::{
    Instance, InstanceHandle, InstanceInternal, State, TerminationDetails, CURRENT_INSTANCE,
    HOST_CTX,
};
use crate::WASM_PAGE_SIZE;
use libc::c_void;
use std::sync::Once;

/// Marker type for the `vmctx` pointer argument.
///
/// This type should only be used with [`Vmctx::from_raw()`](struct.Vmctx.html#method.from_raw).
#[repr(C)]
pub struct lucet_vmctx {
    _unused: [u8; 0],
}

/// An opaque handle to a running instance's context.
#[derive(Debug)]
pub struct Vmctx {
    vmctx: *mut lucet_vmctx,
}

impl Vmctx {
    /// Create a `Vmctx` from the compiler-inserted `vmctx` argument in a guest
    /// function.
    pub unsafe fn from_raw(vmctx: *mut lucet_vmctx) -> Vmctx {
        let res = Vmctx { vmctx };
        // we don't actually need the instance for this call, but asking for it here causes an
        // earlier failure if the pointer isn't valid
        assert!(res.instance().valid_magic());
        res
    }

    /// Return the underlying `vmctx` pointer.
    pub fn as_raw(&self) -> *mut lucet_vmctx {
        self.vmctx
    }

    /// Get a reference to the `Instance` for this guest.
    fn instance(&self) -> &Instance {
        unsafe { Instance::from_vmctx(self.vmctx) }
    }

    /// Get a mutable reference to the `Instance` for this guest.
    fn instance_mut(&mut self) -> &mut Instance {
        unsafe { Instance::from_vmctx(self.vmctx) }
    }

    /// Return the WebAssembly heap as a slice of bytes.
    pub fn heap(&self) -> &[u8] {
        self.instance().heap()
    }

    /// Return the WebAssembly heap as a mutable slice of bytes.
    pub fn heap_mut(&mut self) -> &mut [u8] {
        self.instance_mut().heap_mut()
    }

    /// Check whether a given range in the host address space overlaps with the memory that backs
    /// the instance heap.
    pub fn check_heap<T>(&self, ptr: *const T, len: usize) -> bool {
        self.instance().check_heap(ptr, len)
    }

    /// Get the embedder context for this instance.
    pub fn embed_ctx(&self) -> *mut c_void {
        self.instance().embed_ctx
    }

    /// Terminate this guest and return to the host context.
    ///
    /// This will return an `Error::RuntimeTerminated` value to the caller of `Instance::run()`.
    pub fn terminate(&mut self, info: *mut c_void) -> ! {
        unsafe { self.instance_mut().terminate(info) }
    }

    /// Grow the guest memory by the given number of WebAssembly pages.
    ///
    /// On success, returns the number of pages that existed before the call.
    pub fn grow_memory(&mut self, additional_pages: u32) -> Result<u32, Error> {
        self.instance_mut().grow_memory(additional_pages)
    }

    /// Return the WebAssembly globals as a slice of `i64`s.
    pub fn globals(&self) -> &[i64] {
        self.instance().globals()
    }

    /// Return the WebAssembly globals as a mutable slice of `i64`s.
    pub fn globals_mut(&mut self) -> &mut [i64] {
        self.instance_mut().globals_mut()
    }

    /// Get a function pointer by WebAssembly table and function index.
    ///
    /// This is useful when a hostcall takes a function pointer as its argument, as WebAssembly uses
    /// table indices as its runtime representation of function pointers.
    ///
    /// We do not currently reflect function type information into the Rust type system, so callers
    /// of the returned function must take care to cast it to the correct type before calling. The
    /// correct type will include the `vmctx` argument, which the caller is responsible for passing
    /// from its own context.
    ///
    /// ```no_run
    /// use lucet_runtime_internals::vmctx::{lucet_vmctx, Vmctx};
    /// #[no_mangle]
    /// extern "C" fn hostcall_call_binop(
    ///     vmctx: *mut lucet_vmctx,
    ///     binop_table_idx: u32,
    ///     binop_func_idx: u32,
    ///     operand1: u32,
    ///     operand2: u32,
    /// ) -> u32 {
    ///     let mut ctx = unsafe { Vmctx::from_raw(vmctx) };
    ///     if let Ok(binop) = ctx.get_func_from_idx(binop_table_idx, binop_func_idx) {
    ///         let typed_binop = binop as *const extern "C" fn(*mut lucet_vmctx, u32, u32) -> u32;
    ///         unsafe { (*typed_binop)(vmctx, operand1, operand2) }
    ///     } else {
    ///         // invalid function index
    ///         ctx.terminate(std::ptr::null_mut())
    ///     }
    /// }
    pub fn get_func_from_idx(
        &self,
        table_idx: u32,
        func_idx: u32,
    ) -> Result<*const extern "C" fn(), Error> {
        self.instance()
            .module()
            .get_func_from_idx(table_idx, func_idx)
    }
}

impl Instance {
    /// Get an Instance from the `vmctx` pointer.
    ///
    /// Only safe to call from within the guest context.
    unsafe fn from_vmctx<'a>(vmctx: *mut lucet_vmctx) -> &'a mut Instance {
        assert!(!vmctx.is_null(), "vmctx is not null");

        let inst_ptr = (vmctx as usize - instance_heap_offset()) as *mut Instance;

        // We shouldn't actually need to access the thread local, only the exception handler should
        // need to. But, as long as the thread local exists, we should make sure that the guest
        // hasn't pulled any shenanigans and passed a bad vmctx. (Codegen should ensure the guest
        // cant pull any shenanigans but there have been bugs before.)
        CURRENT_INSTANCE.with(|current_instance| {
            if let Some(current_inst_ptr) = current_instance.borrow().map(|nn| nn.as_ptr()) {
                assert_eq!(
                    inst_ptr, current_inst_ptr,
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
        self.state = State::Terminated {
            details: TerminationDetails { info },
        };
        HOST_CTX.with(|host_ctx| Context::set(&*host_ctx.get()))
    }
}

/// Unsafely get a `Vmctx` from an `InstanceHandle`, and fake a current instance TLS variable.
///
/// This is provided for compatibility with the Terrarium memory management test suite, but should
/// absolutely not be used in newer code.
#[deprecated]
pub unsafe fn vmctx_from_mock_instance(inst: &InstanceHandle) -> Vmctx {
    CURRENT_INSTANCE.with(|current_instance| {
        let mut current_instance = current_instance.borrow_mut();
        *current_instance = Some(std::ptr::NonNull::new_unchecked(
            inst.alloc().slot().start as *mut Instance,
        ));
    });
    Vmctx::from_raw(inst.alloc().slot().heap as *mut lucet_vmctx)
}

static VMCTX_CAPI_INIT: Once = Once::new();

/// Should never actually be called, but should be reachable via a trait method to prevent DCE.
pub fn vmctx_capi_init() {
    use std::ptr::read_volatile;
    VMCTX_CAPI_INIT.call_once(|| unsafe {
        read_volatile(lucet_vmctx_get_heap as *const extern "C" fn());
        read_volatile(lucet_vmctx_get_globals as *const extern "C" fn());
        read_volatile(lucet_vmctx_current_memory as *const extern "C" fn());
        read_volatile(lucet_vmctx_grow_memory as *const extern "C" fn());
        read_volatile(lucet_vmctx_check_heap as *const extern "C" fn());
        read_volatile(lucet_vmctx_terminate as *const extern "C" fn());
        read_volatile(lucet_vmctx_get_delegate as *const extern "C" fn());
        read_volatile(lucet_vmctx_get_func_from_idx as *const extern "C" fn());
        read_volatile(crate::probestack::lucet_probestack as *const c_void);
    });
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_heap(vmctx: *mut lucet_vmctx) -> *mut u8 {
    Vmctx::from_raw(vmctx).instance().alloc().slot().heap as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_globals(vmctx: *mut lucet_vmctx) -> *mut i64 {
    Vmctx::from_raw(vmctx).instance().alloc().slot().globals as *mut i64
}

/// Get the number of WebAssembly pages currently in the heap.
#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_current_memory(vmctx: *mut lucet_vmctx) -> libc::uint32_t {
    Vmctx::from_raw(vmctx).instance().alloc().heap_len() as u32 / WASM_PAGE_SIZE
}

#[no_mangle]
/// Grows the guest heap by the given number of WebAssembly pages.
///
/// On success, returns the number of pages that existed before the call. On failure, returns `-1`.
pub unsafe extern "C" fn lucet_vmctx_grow_memory(
    vmctx: *mut lucet_vmctx,
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
    vmctx: *mut lucet_vmctx,
    ptr: *mut c_void,
    len: libc::size_t,
) -> bool {
    let inst = Instance::from_vmctx(vmctx);
    inst.check_heap(ptr, len)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_func_from_idx(
    vmctx: *mut lucet_vmctx,
    table_idx: u32,
    func_idx: u32,
) -> *const c_void {
    let inst = Instance::from_vmctx(vmctx);
    inst.module()
        .get_func_from_idx(table_idx, func_idx)
        // the Rust API actually returns a pointer to a function pointer, so we want to dereference
        // one layer of that to make it nicer in C
        .map(|fptr| *(fptr as *const *const c_void))
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_terminate(vmctx: *mut lucet_vmctx, info: *mut c_void) {
    let inst = Instance::from_vmctx(vmctx);
    inst.terminate(info);
}

#[no_mangle]
/// Get the delegate object for the current instance.
///
/// TODO: rename
pub unsafe extern "C" fn lucet_vmctx_get_delegate(vmctx: *mut lucet_vmctx) -> *mut c_void {
    let inst = Instance::from_vmctx(vmctx);
    inst.embed_ctx
}
