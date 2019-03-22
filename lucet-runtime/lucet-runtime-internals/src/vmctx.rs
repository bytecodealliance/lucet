//! Interfaces for accessing instance data from hostcalls.
//!
//! This module contains both a Rust-friendly API ([`Vmctx`](struct.Vmctx.html)) as well as C-style
//! exports for compatibility with hostcalls written against `lucet-runtime-c`.

pub use crate::c_api::lucet_vmctx;

use crate::alloc::instance_heap_offset;
use crate::context::Context;
use crate::error::Error;
use crate::instance::{
    Instance, InstanceHandle, InstanceInternal, State, TerminationDetails, CURRENT_INSTANCE,
    HOST_CTX,
};
use std::any::Any;

/// An opaque handle to a running instance's context.
#[derive(Debug)]
pub struct Vmctx {
    vmctx: *mut lucet_vmctx,
}

pub trait VmctxInternal {
    /// Get a reference to the `Instance` for this guest.
    fn instance(&self) -> &Instance;

    /// Get a mutable reference to the `Instance` for this guest.
    ///
    /// ### Safety
    ///
    /// Using this method, you could hold on to multiple mutable references to the same
    /// `Instance`. Only use one at a time! This method does not take `&mut self` because otherwise
    /// you could not use orthogonal `&mut` refs that come from `Vmctx`, like the heap or
    /// terminating the instance.
    unsafe fn instance_mut(&self) -> &mut Instance;
}

impl VmctxInternal for Vmctx {
    fn instance(&self) -> &Instance {
        unsafe { instance_from_vmctx(self.vmctx) }
    }

    unsafe fn instance_mut(&self) -> &mut Instance {
        instance_from_vmctx(self.vmctx)
    }
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

    /// Return the WebAssembly heap as a slice of bytes.
    pub fn heap(&self) -> &[u8] {
        self.instance().heap()
    }

    /// Return the WebAssembly heap as a mutable slice of bytes.
    pub fn heap_mut(&mut self) -> &mut [u8] {
        unsafe { self.instance_mut().heap_mut() }
    }

    /// Check whether a given range in the host address space overlaps with the memory that backs
    /// the instance heap.
    pub fn check_heap<T>(&self, ptr: *const T, len: usize) -> bool {
        self.instance().check_heap(ptr, len)
    }

    /// Check whether a context value of a particular type exists.
    pub fn contains_embed_ctx<T: Any>(&self) -> bool {
        self.instance().contains_embed_ctx::<T>()
    }

    /// Get a reference to a context value of a particular type. If it does not exist,
    /// the context will terminate.
    pub fn get_embed_ctx<T: Any>(&self) -> &T {
        unsafe { self.instance_mut().get_embed_ctx_or_term() }
    }

    /// Get a mutable reference to a context value of a particular type> If it does not exist,
    /// the context will terminate.
    pub fn get_embed_ctx_mut<T: Any>(&mut self) -> &mut T {
        unsafe { self.instance_mut().get_embed_ctx_mut_or_term() }
    }

    /// Terminate this guest and return to the host context.
    ///
    /// This will return an `Error::RuntimeTerminated` value to the caller of `Instance::run()`.
    pub fn terminate<I: Any>(&mut self, info: I) -> ! {
        let details = TerminationDetails::provide(info);
        unsafe { self.instance_mut().terminate(details) }
    }

    /// Grow the guest memory by the given number of WebAssembly pages.
    ///
    /// On success, returns the number of pages that existed before the call.
    pub fn grow_memory(&mut self, additional_pages: u32) -> Result<u32, Error> {
        unsafe { self.instance_mut().grow_memory(additional_pages) }
    }

    /// Return the WebAssembly globals as a slice of `i64`s.
    pub fn globals(&self) -> &[i64] {
        self.instance().globals()
    }

    /// Return the WebAssembly globals as a mutable slice of `i64`s.
    pub fn globals_mut(&mut self) -> &mut [i64] {
        unsafe { self.instance_mut().globals_mut() }
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
    ///         ctx.terminate("invalid function index")
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

/// Terminating an instance requires mutating the state field, and then jumping back to the
/// host context. The mutable borrow may conflict with a mutable borrow of the embed_ctx if
/// this is performed via a method call. We use a macro so we can convince the borrow checker that
/// this is safe at each use site.
macro_rules! inst_terminate {
    ($self:ident, $details:expr) => {{
        $self.state = State::Terminated { details: $details };
        #[allow(unused_unsafe)] // The following unsafe will be incorrectly warned as unused
        HOST_CTX.with(|host_ctx| unsafe { Context::set(&*host_ctx.get()) })
    }};
}

/// Get an `Instance` from the `vmctx` pointer.
///
/// Only safe to call from within the guest context.
pub unsafe fn instance_from_vmctx<'a>(vmctx: *mut lucet_vmctx) -> &'a mut Instance {
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

impl Instance {
    /// Helper function specific to Vmctx::get_embed_ctx. From the vmctx interface,
    /// there is no way to recover if the expected embedder ctx is not set, so we terminate
    /// the instance.
    fn get_embed_ctx_or_term<T: Any>(&mut self) -> &T {
        match self.embed_ctx.get::<T>() {
            Some(t) => t,
            None => inst_terminate!(self, TerminationDetails::GetEmbedCtx),
        }
    }

    /// Helper function specific to Vmctx::get_embed_ctx_mut. See above.
    fn get_embed_ctx_mut_or_term<T: Any>(&mut self) -> &mut T {
        match self.embed_ctx.get_mut::<T>() {
            Some(t) => t,
            None => inst_terminate!(self, TerminationDetails::GetEmbedCtx),
        }
    }

    /// Terminate the guest and swap back to the host context.
    ///
    /// Only safe to call from within the guest context.
    unsafe fn terminate(&mut self, details: TerminationDetails) -> ! {
        inst_terminate!(self, details)
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
