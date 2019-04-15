mod siginfo_ext;
pub mod signals;

pub use crate::instance::signals::{signal_handler_none, SignalBehavior, SignalHandler};

use crate::alloc::Alloc;
use crate::context::Context;
use crate::embed_ctx::CtxMap;
use crate::error::Error;
use crate::instance::siginfo_ext::SiginfoExt;
use crate::module::{self, Global, Module};
use crate::sysdeps::UContext;
use crate::trapcode::{TrapCode, TrapCodeType};
use crate::val::{UntypedRetVal, Val};
use crate::WASM_PAGE_SIZE;
use libc::{c_void, siginfo_t, uintptr_t, SIGBUS, SIGSEGV};
use std::any::Any;
use std::cell::{RefCell, UnsafeCell};
use std::ffi::{CStr, CString};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::sync::Arc;

pub const LUCET_INSTANCE_MAGIC: u64 = 746932922;
pub const INSTANCE_PADDING: usize = 2328;

thread_local! {
    /// The host context.
    ///
    /// Control returns here implicitly due to the setup in `Context::init()` when guest functions
    /// return normally. Control can return here explicitly from signal handlers when the guest
    /// program needs to be terminated.
    ///
    /// This is an `UnsafeCell` due to nested borrows. The context must be borrowed mutably when
    /// swapping to the guest context, which means that borrow exists for the entire time the guest
    /// function runs even though the mutation to the host context is done only at the beginning of
    /// the swap. Meanwhile, the signal handler can run at any point during the guest function, and
    /// so it also must be able to immutably borrow the host context if it needs to swap back. The
    /// runtime borrowing constraints for a `RefCell` are therefore too strict for this variable.
    pub(crate) static HOST_CTX: UnsafeCell<Context> = UnsafeCell::new(Context::new());

    /// The currently-running `Instance`, if one exists.
    pub(crate) static CURRENT_INSTANCE: RefCell<Option<NonNull<Instance>>> = RefCell::new(None);
}

/// A smart pointer to an [`Instance`](struct.Instance.html) that properly manages cleanup when dropped.
///
/// Instances are always stored in memory backed by a `Region`; we never want to create one directly
/// with the Rust allocator. This type allows us to abide by that rule while also having an owned
/// type that cleans up the instance when we are done with it.
///
/// Since this type implements `Deref` and `DerefMut` to `Instance`, it can usually be treated as
/// though it were a `&mut Instance`.
pub struct InstanceHandle {
    inst: NonNull<Instance>,
}

// raw pointer lint
unsafe impl Send for InstanceHandle {}

/// Create a new `InstanceHandle`.
///
/// This is not meant for public consumption, but rather is used to make implementations of
/// `Region`.
///
/// # Safety
///
/// This function runs the guest code for the WebAssembly `start` section, and running any guest
/// code is potentially unsafe; see [`Instance::run()`](struct.Instance.html#method.run).
pub fn new_instance_handle(
    instance: *mut Instance,
    module: Arc<dyn Module>,
    alloc: Alloc,
    embed_ctx: CtxMap,
) -> Result<InstanceHandle, Error> {
    let inst = NonNull::new(instance)
        .ok_or(lucet_format_err!("instance pointer is null; this is a bug"))?;

    // do this check first so we don't run `InstanceHandle::drop()` for a failure
    lucet_ensure!(
        unsafe { inst.as_ref().magic } != LUCET_INSTANCE_MAGIC,
        "created a new instance handle in memory with existing instance magic; this is a bug"
    );

    let mut handle = InstanceHandle { inst };

    let inst = Instance::new(alloc, module, embed_ctx);

    unsafe {
        // this is wildly unsafe! you must be very careful to not let the drop impls run on the
        // uninitialized fields; see
        // <https://doc.rust-lang.org/std/mem/fn.forget.html#use-case-1>

        // write the whole struct into place over the uninitialized page
        ptr::write(&mut *handle, inst);
    };

    handle.reset()?;

    Ok(handle)
}

pub fn instance_handle_to_raw(inst: InstanceHandle) -> *mut Instance {
    let ptr = inst.inst.as_ptr();
    std::mem::forget(inst);
    ptr
}

pub unsafe fn instance_handle_from_raw(ptr: *mut Instance) -> InstanceHandle {
    InstanceHandle {
        inst: NonNull::new_unchecked(ptr),
    }
}

// Safety argument for these deref impls: the instance's `Alloc` field contains an `Arc` to the
// region that backs this memory, keeping the page containing the `Instance` alive as long as the
// region exists

impl Deref for InstanceHandle {
    type Target = Instance;
    fn deref(&self) -> &Self::Target {
        unsafe { self.inst.as_ref() }
    }
}

impl DerefMut for InstanceHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inst.as_mut() }
    }
}

impl Drop for InstanceHandle {
    fn drop(&mut self) {
        // eprintln!("InstanceHandle::drop()");
        // zero out magic, then run the destructor by taking and dropping the inner `Instance`
        self.magic = 0;
        unsafe {
            mem::replace(self.inst.as_mut(), mem::uninitialized());
        }
    }
}

/// A Lucet program, together with its dedicated memory and signal handlers.
///
/// This is the primary interface for running programs, examining return values, and accessing the
/// WebAssembly heap.
///
/// `Instance`s are never created by runtime users directly, but rather are acquired from
/// [`Region`](trait.Region.html)s and often accessed through
/// [`InstanceHandle`](struct.InstanceHandle.html) smart pointers. This guarantees that instances
/// and their fields are never moved in memory, otherwise raw pointers in the metadata could be
/// unsafely invalidated.
#[repr(C)]
pub struct Instance {
    /// Used to catch bugs in pointer math used to find the address of the instance
    magic: u64,

    /// The embedding context is a map containing embedder-specific values that are used to
    /// implement hostcalls
    pub(crate) embed_ctx: CtxMap,

    /// The program (WebAssembly module) that is the entrypoint for the instance.
    module: Arc<dyn Module>,

    /// The `Context` in which the guest program runs
    ctx: Context,

    /// Instance state and error information
    pub(crate) state: State,

    /// The memory allocated for this instance
    alloc: Alloc,

    /// Handler run for signals that do not arise from a known WebAssembly trap, or that involve
    /// memory outside of the current instance.
    fatal_handler: fn(&Instance) -> !,

    /// A fatal handler set from C
    c_fatal_handler: Option<unsafe extern "C" fn(*mut Instance)>,

    /// Handler run when `SIGBUS`, `SIGFPE`, `SIGILL`, or `SIGSEGV` are caught by the instance thread.
    signal_handler: Box<
        dyn Fn(
            &Instance,
            &TrapCode,
            libc::c_int,
            *const siginfo_t,
            *const c_void,
        ) -> SignalBehavior,
    >,

    /// Pointer to the function used as the entrypoint (for use in backtraces)
    entrypoint: *const extern "C" fn(),

    /// Padding to ensure the pointer to globals at the end of the page occupied by the `Instance`
    _reserved: [u8; INSTANCE_PADDING],

    /// Pointer to the globals
    ///
    /// This is accessed through the `vmctx` pointer, which points to the heap that begins
    /// immediately after this struct, so it has to come at the very end.
    globals_ptr: *const i64,
}

/// APIs that are internal, but useful to implementors of extension modules; you probably don't want
/// this trait!
///
/// This is a trait rather than inherent `impl`s in order to keep the `lucet-runtime` API clean and
/// safe.
pub trait InstanceInternal {
    fn alloc(&self) -> &Alloc;
    fn alloc_mut(&mut self) -> &mut Alloc;
    fn module(&self) -> &dyn Module;
    fn state(&self) -> &State;
    fn valid_magic(&self) -> bool;
}

impl InstanceInternal for Instance {
    /// Get a reference to the instance's `Alloc`.
    fn alloc(&self) -> &Alloc {
        &self.alloc
    }

    /// Get a mutable reference to the instance's `Alloc`.
    fn alloc_mut(&mut self) -> &mut Alloc {
        &mut self.alloc
    }

    /// Get a reference to the instance's `Module`.
    fn module(&self) -> &dyn Module {
        self.module.deref()
    }

    /// Get a reference to the instance's `State`.
    fn state(&self) -> &State {
        &self.state
    }

    /// Check whether the instance magic is valid.
    fn valid_magic(&self) -> bool {
        self.magic == LUCET_INSTANCE_MAGIC
    }
}

// Public API
impl Instance {
    /// Run a function with arguments in the guest context at the given entrypoint.
    ///
    /// ```no_run
    /// # use lucet_runtime_internals::instance::InstanceHandle;
    /// # let instance: InstanceHandle = unimplemented!();
    /// // regular execution yields `Ok(UntypedRetVal)`
    /// let retval = instance.run(b"factorial", &[5u64.into()]).unwrap();
    /// assert_eq!(u64::from(retval), 120u64);
    ///
    /// // runtime faults yield `Err(Error)`
    /// let result = instance.run(b"faulting_function", &[]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// # Safety
    ///
    /// This is unsafe in two ways:
    ///
    /// - The type of the entrypoint might not be correct. It might take a different number or
    /// different types of arguments than are provided to `args`. It might not even point to a
    /// function! We will likely add type information to `lucetc` output so we can dynamically check
    /// the type in the future.
    ///
    /// - The entrypoint is foreign code. While we may be convinced that WebAssembly compiled to
    /// native code by `lucetc` is safe, we do not have the same guarantee for the hostcalls that a
    /// guest may invoke. They might be implemented in an unsafe language, so we must treat this
    /// call as unsafe, just like any other FFI call.
    ///
    /// For the moment, we do not mark this as `unsafe` in the Rust type system, but that may change
    /// in the future.
    pub fn run(&mut self, entrypoint: &[u8], args: &[Val]) -> Result<UntypedRetVal, Error> {
        let func = self.module.get_export_func(entrypoint)?;
        self.run_func(func, &args)
    }

    /// Run a function with arguments in the guest context from the [WebAssembly function
    /// table](https://webassembly.github.io/spec/core/syntax/modules.html#tables).
    ///
    /// The same safety caveats of [`Instance::run()`](struct.Instance.html#method.run) apply.
    pub fn run_func_idx(
        &mut self,
        table_idx: u32,
        func_idx: u32,
        args: &[Val],
    ) -> Result<UntypedRetVal, Error> {
        let func = self.module.get_func_from_idx(table_idx, func_idx)?;
        self.run_func(func, &args)
    }

    /// Reset the instance's heap and global variables to their initial state.
    ///
    /// The WebAssembly `start` section will also be run, if one exists.
    ///
    /// The embedder contexts present at instance creation or added with
    /// [`Instance::insert_embed_ctx()`](struct.Instance.html#method.insert_embed_ctx) are not
    /// modified by this call; it is the embedder's responsibility to clear or reset their state if
    /// necessary.
    ///
    /// # Safety
    ///
    /// This function runs the guest code for the WebAssembly `start` section, and running any guest
    /// code is potentially unsafe; see [`Instance::run()`](struct.Instance.html#method.run).
    pub fn reset(&mut self) -> Result<(), Error> {
        self.alloc.reset_heap(self.module.as_ref())?;
        let globals = unsafe { self.alloc.globals_mut() };
        let mod_globals = self.module.globals();
        for (i, v) in mod_globals.iter().enumerate() {
            globals[i] = match v.global() {
                Global::Import { .. } => {
                    return Err(Error::Unsupported(format!(
                        "global imports are unsupported; found: {:?}",
                        i
                    )));
                }
                Global::Def { def } => def.init_val(),
            };
        }

        self.state = State::Ready {
            retval: UntypedRetVal::default(),
        };

        self.run_start()?;

        Ok(())
    }

    /// Grow the guest memory by the given number of WebAssembly pages.
    ///
    /// On success, returns the number of pages that existed before the call.
    pub fn grow_memory(&mut self, additional_pages: u32) -> Result<u32, Error> {
        let orig_len = self
            .alloc
            .expand_heap(additional_pages * WASM_PAGE_SIZE, self.module.as_ref())?;
        Ok(orig_len / WASM_PAGE_SIZE)
    }

    /// Return the WebAssembly heap as a slice of bytes.
    pub fn heap(&self) -> &[u8] {
        unsafe { self.alloc.heap() }
    }

    /// Return the WebAssembly heap as a mutable slice of bytes.
    pub fn heap_mut(&mut self) -> &mut [u8] {
        unsafe { self.alloc.heap_mut() }
    }

    /// Return the WebAssembly heap as a slice of `u32`s.
    pub fn heap_u32(&self) -> &[u32] {
        unsafe { self.alloc.heap_u32() }
    }

    /// Return the WebAssembly heap as a mutable slice of `u32`s.
    pub fn heap_u32_mut(&mut self) -> &mut [u32] {
        unsafe { self.alloc.heap_u32_mut() }
    }

    /// Return the WebAssembly globals as a slice of `i64`s.
    pub fn globals(&self) -> &[i64] {
        unsafe { self.alloc.globals() }
    }

    /// Return the WebAssembly globals as a mutable slice of `i64`s.
    pub fn globals_mut(&mut self) -> &mut [i64] {
        unsafe { self.alloc.globals_mut() }
    }

    /// Check whether a given range in the host address space overlaps with the memory that backs
    /// the instance heap.
    pub fn check_heap<T>(&self, ptr: *const T, len: usize) -> bool {
        self.alloc.mem_in_heap(ptr, len)
    }

    /// Check whether a context value of a particular type exists.
    pub fn contains_embed_ctx<T: Any>(&self) -> bool {
        self.embed_ctx.contains::<T>()
    }

    /// Get a reference to a context value of a particular type, if it exists.
    pub fn get_embed_ctx<T: Any>(&self) -> Option<&T> {
        self.embed_ctx.get::<T>()
    }

    /// Get a mutable reference to a context value of a particular type, if it exists.
    pub fn get_embed_ctx_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.embed_ctx.get_mut::<T>()
    }

    /// Insert a context value.
    ///
    /// If a context value of the same type already existed, it is returned.
    ///
    /// **Note**: this method is intended for embedder contexts that need to be added _after_ an
    /// instance is created and initialized. To add a context for an instance's entire lifetime,
    /// including the execution of its `start` section, see
    /// [`Region::new_instance_builder()`](trait.Region.html#method.new_instance_builder).
    pub fn insert_embed_ctx<T: Any>(&mut self, x: T) -> Option<T> {
        self.embed_ctx.insert(x)
    }

    /// Remove a context value of a particular type, returning it if it exists.
    pub fn remove_embed_ctx<T: Any>(&mut self) -> Option<T> {
        self.embed_ctx.remove::<T>()
    }

    /// Set the handler run when `SIGBUS`, `SIGFPE`, `SIGILL`, or `SIGSEGV` are caught by the
    /// instance thread.
    ///
    /// In most cases, these signals are unrecoverable for the instance that raised them, but do not
    /// affect the rest of the process.
    ///
    /// The default signal handler returns
    /// [`SignalBehavior::Default`](enum.SignalBehavior.html#variant.Default), which yields a
    /// runtime fault error.
    ///
    /// The signal handler must be
    /// [signal-safe](http://man7.org/linux/man-pages/man7/signal-safety.7.html).
    pub fn set_signal_handler<H>(&mut self, handler: H)
    where
        H: 'static
            + Fn(&Instance, &TrapCode, libc::c_int, *const siginfo_t, *const c_void) -> SignalBehavior,
    {
        self.signal_handler = Box::new(handler) as Box<SignalHandler>;
    }

    /// Set the handler run for signals that do not arise from a known WebAssembly trap, or that
    /// involve memory outside of the current instance.
    ///
    /// Fatal signals are not only unrecoverable for the instance that raised them, but may
    /// compromise the correctness of the rest of the process if unhandled.
    ///
    /// The default fatal handler calls `panic!()`.
    pub fn set_fatal_handler(&mut self, handler: fn(&Instance) -> !) {
        self.fatal_handler = handler;
    }

    /// Set the fatal handler to a C-compatible function.
    ///
    /// This is a separate interface, because C functions can't return the `!` type. Like the
    /// regular `fatal_handler`, it is not expected to return, but we cannot enforce that through
    /// types.
    ///
    /// When a fatal error occurs, this handler is run first, and then the regular `fatal_handler`
    /// runs in case it returns.
    pub fn set_c_fatal_handler(&mut self, handler: unsafe extern "C" fn(*mut Instance)) {
        self.c_fatal_handler = Some(handler);
    }
}

// Private API
impl Instance {
    fn new(alloc: Alloc, module: Arc<dyn Module>, embed_ctx: CtxMap) -> Self {
        let globals_ptr = alloc.slot().globals as *mut i64;
        Instance {
            magic: LUCET_INSTANCE_MAGIC,
            embed_ctx: embed_ctx,
            module,
            ctx: Context::new(),
            state: State::Ready {
                retval: UntypedRetVal::default(),
            },
            alloc,
            fatal_handler: default_fatal_handler,
            c_fatal_handler: None,
            signal_handler: Box::new(signal_handler_none) as Box<SignalHandler>,
            entrypoint: ptr::null(),
            _reserved: [0; INSTANCE_PADDING],
            globals_ptr,
        }
    }

    /// Run a function in guest context at the given entrypoint.
    fn run_func(
        &mut self,
        func: *const extern "C" fn(),
        args: &[Val],
    ) -> Result<UntypedRetVal, Error> {
        lucet_ensure!(
            self.state.is_ready(),
            "instance must be ready; this is a bug"
        );
        if func.is_null() {
            return Err(Error::InvalidArgument(
                "entrypoint function cannot be null; this is probably a malformed module",
            ));
        }
        self.entrypoint = func;

        let mut args_with_vmctx = vec![Val::from(self.alloc.slot().heap)];
        args_with_vmctx.extend_from_slice(args);

        HOST_CTX.with(|host_ctx| {
            Context::init(
                unsafe { self.alloc.stack_u64_mut() },
                unsafe { &mut *host_ctx.get() },
                &mut self.ctx,
                func,
                &args_with_vmctx,
            )
        })?;

        self.state = State::Running;

        // there should never be another instance running on this thread when we enter this function
        CURRENT_INSTANCE.with(|current_instance| {
            let mut current_instance = current_instance.borrow_mut();
            assert!(
                current_instance.is_none(),
                "no other instance is running on this thread"
            );
            // safety: `self` is not null if we are in this function
            *current_instance = Some(unsafe { NonNull::new_unchecked(self) });
        });

        self.with_signals_on(|i| {
            HOST_CTX.with(|host_ctx| {
                // Save the current context into `host_ctx`, and jump to the guest context. The
                // lucet context is linked to host_ctx, so it will return here after it finishes,
                // successfully or otherwise.
                unsafe { Context::swap(&mut *host_ctx.get(), &mut i.ctx) };
                Ok(())
            })
        })?;

        CURRENT_INSTANCE.with(|current_instance| {
            *current_instance.borrow_mut() = None;
        });

        // Sandbox has jumped back to the host process, indicating it has either:
        //
        // * trapped, or called hostcall_error: state tag changed to something other than `Running`
        // * function body returned: set state back to `Ready` with return value

        match &self.state {
            State::Running => {
                let retval = self.ctx.get_untyped_retval();
                self.state = State::Ready { retval };
                Ok(retval)
            }
            State::Terminated { details, .. } => Err(Error::RuntimeTerminated(details.clone())),
            State::Fault { .. } => {
                // Sandbox is no longer runnable. It's unsafe to determine all error details in the signal
                // handler, so we fill in extra details here.
                self.populate_fault_detail()?;
                if let State::Fault { ref details, .. } = self.state {
                    if details.fatal {
                        // Some errors indicate that the guest is not functioning correctly or that
                        // the loaded code violated some assumption, so bail out via the fatal
                        // handler.

                        // Run the C-style fatal handler, if it exists.
                        self.c_fatal_handler
                            .map(|h| unsafe { h(self as *mut Instance) });

                        // If there is no C-style fatal handler, or if it (erroneously) returns,
                        // call the Rust handler that we know will not return
                        (self.fatal_handler)(self)
                    } else {
                        // leave the full fault details in the instance state, and return the
                        // higher-level info to the user
                        Err(Error::RuntimeFault(details.clone()))
                    }
                } else {
                    panic!("state remains Fault after populate_fault_detail()")
                }
            }
            State::Ready { .. } => {
                panic!("instance in Ready state after returning from guest context")
            }
        }
    }

    fn run_start(&mut self) -> Result<(), Error> {
        if let Some(start) = self.module.get_start_func()? {
            self.run_func(start, &[])?;
        }
        Ok(())
    }

    fn populate_fault_detail(&mut self) -> Result<(), Error> {
        if let State::Fault {
            details:
                FaultDetails {
                    rip_addr,
                    trapcode,
                    ref mut fatal,
                    ref mut rip_addr_details,
                    ..
                },
            siginfo,
            ..
        } = self.state
        {
            // We do this after returning from the signal handler because it requires `dladdr`
            // calls, which are not signal safe
            *rip_addr_details = self.module.addr_details(rip_addr as *const c_void)?.clone();

            // If the trap table lookup returned unknown, it is a fatal error
            let unknown_fault = trapcode.ty == TrapCodeType::Unknown;

            // If the trap was a segv or bus fault and the addressed memory was outside the
            // guard pages, it is also a fatal error
            let outside_guard = (siginfo.si_signo == SIGSEGV || siginfo.si_signo == SIGBUS)
                && !self.alloc.addr_in_heap_guard(siginfo.si_addr());

            *fatal = unknown_fault || outside_guard;
        }
        Ok(())
    }
}

pub enum State {
    Ready {
        retval: UntypedRetVal,
    },
    Running,
    Fault {
        details: FaultDetails,
        siginfo: libc::siginfo_t,
        context: UContext,
    },
    Terminated {
        details: TerminationDetails,
    },
}

/// Information about a runtime fault.
///
/// Runtime faults are raised implictly by signal handlers that return `SignalBehavior::Default` in
/// response to signals arising while a guest is running.
#[derive(Clone, Debug)]
pub struct FaultDetails {
    /// If true, the instance's `fatal_handler` will be called.
    pub fatal: bool,
    /// Information about the type of fault that occurred.
    pub trapcode: TrapCode,
    /// The instruction pointer where the fault occurred.
    pub rip_addr: uintptr_t,
    /// Extra information about the instruction pointer's location, if available.
    pub rip_addr_details: Option<module::AddrDetails>,
}

impl std::fmt::Display for FaultDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.fatal {
            write!(f, "fault FATAL ")?;
        } else {
            write!(f, "fault ")?;
        }

        self.trapcode.fmt(f)?;

        write!(f, "code at address {:p}", self.rip_addr as *const c_void)?;

        if let Some(ref addr_details) = self.rip_addr_details {
            if let Some(ref fname) = addr_details.file_name {
                let sname = addr_details
                    .sym_name
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("<unknown>");
                write!(f, " (symbol {}:{})", fname, sname)?;
            }
            if addr_details.in_module_code {
                write!(f, " (inside module code)")
            } else {
                write!(f, " (not inside module code)")
            }
        } else {
            write!(f, " (unknown whether in module)")
        }
    }
}

/// Information about a terminated guest.
///
/// Guests are terminated either explicitly by `Vmctx::terminate()`, or implicitly by signal
/// handlers that return `SignalBehavior::Terminate`. It usually indicates that an unrecoverable
/// error has occurred in a hostcall, rather than in WebAssembly code.
#[derive(Clone)]
pub enum TerminationDetails {
    Signal,
    GetEmbedCtx,
    /// Calls to `Vmctx::terminate()` may attach an arbitrary pointer for extra debugging
    /// information.
    Provided(Arc<dyn Any>),
}

impl TerminationDetails {
    pub fn provide<A: Any>(details: A) -> Self {
        TerminationDetails::Provided(Arc::new(details))
    }
    pub fn provided_details(&self) -> Option<&dyn Any> {
        match self {
            TerminationDetails::Provided(a) => Some(a.as_ref()),
            _ => None,
        }
    }
}

// Because of deref coercions, the code above was tricky to get right-
// test that a string makes it through
#[test]
fn termination_details_any_typing() {
    let hello = "hello, world".to_owned();
    let details = TerminationDetails::provide(hello.clone());
    let provided = details.provided_details().expect("got Provided");
    assert_eq!(
        provided.downcast_ref::<String>().expect("right type"),
        &hello
    );
}

impl std::fmt::Debug for TerminationDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "TerminationDetails::{}",
            match self {
                TerminationDetails::Signal => "Signal",
                TerminationDetails::GetEmbedCtx => "GetEmbedCtx",
                TerminationDetails::Provided(_) => "Provided(Any)",
            }
        )
    }
}

unsafe impl Send for TerminationDetails {}
unsafe impl Sync for TerminationDetails {}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            State::Ready { .. } => write!(f, "ready"),
            State::Running => write!(f, "running"),
            State::Fault {
                details, siginfo, ..
            } => {
                write!(f, "{}", details)?;
                write!(
                    f,
                    " triggered by {}: ",
                    strsignal_wrapper(siginfo.si_signo)
                        .into_string()
                        .expect("strsignal returns valid UTF-8")
                )?;

                if siginfo.si_signo == SIGSEGV || siginfo.si_signo == SIGBUS {
                    // We know this is inside the heap guard, because by the time we get here,
                    // `lucet_error_verify_trap_safety` will have run and validated it.
                    write!(
                        f,
                        " accessed memory at {:p} (inside heap guard)",
                        siginfo.si_addr()
                    )?;
                }
                Ok(())
            }
            State::Terminated { .. } => write!(f, "terminated"),
        }
    }
}

impl State {
    pub fn is_ready(&self) -> bool {
        if let State::Ready { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_running(&self) -> bool {
        if let State::Running = self {
            true
        } else {
            false
        }
    }

    pub fn is_fault(&self) -> bool {
        if let State::Fault { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_fatal(&self) -> bool {
        if let State::Fault {
            details: FaultDetails { fatal, .. },
            ..
        } = self
        {
            *fatal
        } else {
            false
        }
    }

    pub fn is_terminated(&self) -> bool {
        if let State::Terminated { .. } = self {
            true
        } else {
            false
        }
    }
}

fn default_fatal_handler(inst: &Instance) -> ! {
    panic!("> instance {:p} had fatal error: {}", inst, inst.state);
}

// TODO: PR into `libc`
extern "C" {
    #[no_mangle]
    fn strsignal(sig: libc::c_int) -> *mut libc::c_char;
}

// TODO: PR into `nix`
fn strsignal_wrapper(sig: libc::c_int) -> CString {
    unsafe { CStr::from_ptr(strsignal(sig)).to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memoffset::offset_of;

    #[test]
    fn instance_size_correct() {
        assert_eq!(mem::size_of::<Instance>(), 4096);
    }

    #[test]
    fn instance_globals_offset_correct() {
        let offset = offset_of!(Instance, globals_ptr) as isize;
        if offset != 4096 - 8 {
            let diff = 4096 - 8 - offset;
            let new_padding = INSTANCE_PADDING as isize + diff;
            panic!("new padding should be: {:?}", new_padding);
        }
        assert_eq!(offset_of!(Instance, globals_ptr), 4096 - 8);
    }
}
