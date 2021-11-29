pub mod execution;
mod siginfo_ext;
pub mod signals;
pub mod state;

pub use crate::instance::execution::{KillError, KillState, KillSuccess, KillSwitch};
pub use crate::instance::signals::{signal_handler_none, SignalBehavior, SignalHandler};
pub use crate::instance::state::State;

use crate::alloc::Alloc;
use crate::context::Context;
use crate::embed_ctx::CtxMap;
use crate::error::Error;
#[cfg(feature = "concurrent_testpoints")]
use crate::lock_testpoints::LockTestpoints;
use crate::module::{self, FunctionHandle, Global, GlobalValue, Module, TrapCode};
use crate::sysdeps::HOST_PAGE_SIZE_EXPECTED;
use crate::val::{UntypedRetVal, Val};
use crate::vmctx::Vmctx;
use crate::WASM_PAGE_SIZE;
use libc::{c_void, pthread_self, siginfo_t, uintptr_t};
use lucet_module::InstanceRuntimeData;
use memoffset::offset_of;
use std::any::Any;
use std::cell::{BorrowError, BorrowMutError, Ref, RefCell, RefMut, UnsafeCell};
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::mem;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::sync::Arc;

pub const LUCET_INSTANCE_MAGIC: u64 = 746_932_922;

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
    needs_inst_drop: bool,
}

// raw pointer lint
unsafe impl Send for InstanceHandle {}

/// Create a new `InstanceHandle`.
///
/// This is not meant for public consumption, but rather is used to make implementations of
/// `Region`.
pub fn new_instance_handle(
    instance: *mut Instance,
    module: Arc<dyn Module>,
    alloc: Alloc,
    embed_ctx: CtxMap,
) -> Result<InstanceHandle, Error> {
    let inst = NonNull::new(instance)
        .ok_or_else(|| lucet_format_err!("instance pointer is null; this is a bug"))?;

    lucet_ensure!(
        unsafe { inst.as_ref().magic } != LUCET_INSTANCE_MAGIC,
        "created a new instance handle in memory with existing instance magic; this is a bug"
    );

    let mut handle = InstanceHandle {
        inst,
        needs_inst_drop: false,
    };

    let inst = Instance::new(alloc, module, embed_ctx);

    unsafe {
        // this is wildly unsafe! you must be very careful to not let the drop impls run on the
        // uninitialized fields; see
        // <https://doc.rust-lang.org/std/mem/fn.forget.html#use-case-1>

        // write the whole struct into place over the uninitialized page
        ptr::write(&mut *handle, inst);
    };

    handle.needs_inst_drop = true;

    handle.reset()?;

    Ok(handle)
}

pub fn instance_handle_to_raw(mut inst: InstanceHandle) -> *mut Instance {
    inst.needs_inst_drop = false;
    inst.inst.as_ptr()
}

pub unsafe fn instance_handle_from_raw(
    ptr: *mut Instance,
    needs_inst_drop: bool,
) -> InstanceHandle {
    InstanceHandle {
        inst: NonNull::new_unchecked(ptr),
        needs_inst_drop,
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
        if self.needs_inst_drop {
            unsafe {
                let inst = self.inst.as_mut();

                // The `inst.alloc` field manages the memory of the instance
                // itself. Note, though, that this field is in a `ManuallyDrop`
                // so it won't get dropped automatically in `drop_in_place`.
                // This is the point where we take over that precise drop.
                //
                // By using `take` here we're basically calling `ptr::read`
                // which "duplicates" the `alloc` since the `alloc` local
                // variable here is the exact same as `inst.alloc`. All we do
                // with `inst`, though, is call `drop_in_place`, which
                // invalidates every other field in `inst`.
                let alloc: Alloc = ManuallyDrop::take(&mut inst.alloc);

                // drop the actual instance
                std::ptr::drop_in_place(inst);

                // Now that we're 100% done with the instance, destructors and
                // all, we can release the memory of the instance back to the
                // original allocator from whence it came (be it mmap or uffd
                // based). This will run the "official" destructor for `Alloc`
                // which internally does the release. Note that after this
                // operation the `inst` pointer is invalid and can no longer be
                // used.
                drop(alloc);
            }
        }
    }
}

/// A Lucet program, together with its dedicated memory and signal handlers.
///
/// This is the primary interface for running programs, examining return values, and accessing the
/// WebAssembly heap.
///
/// `Instance`s are never created by runtime users directly, but rather are acquired from
/// [`Region`](../region/trait.Region.html)s and often accessed through
/// [`InstanceHandle`](../instance/struct.InstanceHandle.html) smart pointers. This guarantees that instances
/// and their fields are never moved in memory, otherwise raw pointers in the metadata could be
/// unsafely invalidated.
///
/// An instance occupies one 4096-byte page in memory, with a layout like:
/// ```text
/// 0xXXXXX000:
///   Instance {
///     .magic
///     .embed_ctx
///      ... etc ...
///   }
///
///   // unused space
///
///   InstanceInternals {
///     .globals
///     .instruction_counter
///   } // last address *inside* `InstanceInternals` is 0xXXXXXFFF
/// 0xXXXXY000: // start of next page, VMContext points here
///   Heap {
///     ..
///   }
/// ```
///
/// This layout allows modules to tightly couple to a handful of fields related to the instance,
/// rather than possibly requiring compiler-side changes (and recompiles) whenever `Instance`
/// changes.
///
/// It also obligates `Instance` to be immediately followed by the heap, but otherwise leaves the
/// locations of the stack, globals, and any other data, to be implementation-defined by the
/// `Region` that actually creates `Slot`s onto which `Instance` are mapped.
/// For information about the layout of all instance-related memory, see the documentation of
/// [MmapRegion](../region/mmap/struct.MmapRegion.html).
#[repr(C)]
#[repr(align(4096))]
pub struct Instance {
    /// Used to catch bugs in pointer math used to find the address of the instance
    magic: u64,

    /// The embedding context is a map containing embedder-specific values that are used to
    /// implement hostcalls
    pub(crate) embed_ctx: CtxMap,

    /// The program (WebAssembly module) that is the entrypoint for the instance.
    pub(crate) module: Arc<dyn Module>,

    /// The `Context` in which the guest program runs
    pub(crate) ctx: Context,

    /// Instance state and error information
    pub(crate) state: State,

    /// Small mutexed state used for remote kill switch functionality
    pub(crate) kill_state: Arc<KillState>,

    #[cfg(feature = "concurrent_testpoints")]
    /// Conditionally-present helpers to force permutations of possible races in testing.
    pub lock_testpoints: Arc<LockTestpoints>,

    /// The memory allocated for this instance.
    ///
    /// Note that this is in a `ManuallyDrop` because this manages the memory of
    /// this `Instance` itself. To have precise control over this memory we
    /// handle this in `Drop for InstanceHandle`.
    alloc: ManuallyDrop<Alloc>,

    /// Handler run for signals that do not arise from a known WebAssembly trap, or that involve
    /// memory outside of the current instance.
    fatal_handler: fn(&Instance) -> !,

    /// A fatal handler set from C
    c_fatal_handler: Option<unsafe extern "C" fn(*mut Instance)>,

    /// Handler run when `SIGBUS`, `SIGFPE`, `SIGILL`, or `SIGSEGV` are caught by the instance thread.
    signal_handler: Box<
        dyn Fn(
            &Instance,
            &Option<TrapCode>,
            libc::c_int,
            *const siginfo_t,
            *const c_void,
        ) -> SignalBehavior,
    >,

    /// Whether to ensure the Lucet signal handler is installed when running this instance.
    ensure_signal_handler_installed: bool,

    /// Whether to install an alternate signal stack while the instance is running.
    ensure_sigstack_installed: bool,

    /// Pointer to the function used as the entrypoint.
    entrypoint: Option<FunctionHandle>,

    /// The value passed back to the guest when resuming a yielded instance.
    pub(crate) resumed_val: Option<Box<dyn Any + 'static>>,

    pub(crate) memory_limiter: Option<Box<dyn MemoryLimiter + Send + Sync + 'static>>,

    /// `_padding` must be the last member of the structure.
    /// This marks where the padding starts to make the structure exactly 4096 bytes long.
    /// It is also used to compute the size of the structure up to that point, i.e. without padding.
    _padding: (),
}

#[async_trait::async_trait]
pub trait MemoryLimiter {
    async fn memory_growing(&mut self, current: usize, desired: usize) -> bool;
    fn memory_grow_failed(&mut self, _error: &Error) {}
}

/// Users of `Instance` must be very careful about when instances are dropped!
///
/// Typically you will not have to worry about this, as InstanceHandle will robustly handle
/// Instance drop semantics. If an instance is dropped, and the Region it's in has already dropped,
/// it may contain the last reference counted pointer to its Region. If so, when Instance's
/// destructor runs, Region will be dropped, and may free or otherwise invalidate the memory that
/// this Instance exists in, *while* the Instance destructor is executing.
impl Drop for Instance {
    fn drop(&mut self) {
        // Reset magic to indicate this instance
        // is no longer valid
        self.magic = 0;
    }
}

/// The result of running or resuming an [`Instance`](struct.Instance.html).
#[derive(Debug)]
pub enum RunResult {
    /// An instance returned with a value.
    ///
    /// The actual type of the contained value depends on the return type of the guest function that
    /// was called. For guest functions with no return value, it is undefined behavior to do
    /// anything with this value.
    Returned(UntypedRetVal),
    /// An instance yielded, potentially with a value.
    ///
    /// This arises when a hostcall invokes one of the
    /// [`Vmctx::yield_*()`](vmctx/struct.Vmctx.html#method.yield_) family of methods. Depending on which
    /// variant is used, the `YieldedVal` may contain a value passed from the guest context to the
    /// host.
    ///
    /// An instance that has yielded may only be resumed
    /// ([with](struct.Instance.html#method.resume_with_val) or
    /// [without](struct.Instance.html#method.resume) a value to returned to the guest),
    /// [reset](struct.Instance.html#method.reset), or dropped. Attempting to run an instance from a
    /// new entrypoint after it has yielded but without first resetting will result in an error.
    Yielded(YieldedVal),
}

impl RunResult {
    /// Try to get a return value from a run result, returning `Error::InstanceNotReturned` if the
    /// instance instead yielded.
    pub fn returned(self) -> Result<UntypedRetVal, Error> {
        match self {
            RunResult::Returned(rv) => Ok(rv),
            RunResult::Yielded(_) => Err(Error::InstanceNotReturned),
        }
    }

    /// Try to get a reference to a return value from a run result, returning
    /// `Error::InstanceNotReturned` if the instance instead yielded.
    pub fn returned_ref(&self) -> Result<&UntypedRetVal, Error> {
        match self {
            RunResult::Returned(rv) => Ok(rv),
            RunResult::Yielded(_) => Err(Error::InstanceNotReturned),
        }
    }

    /// Returns `true` if the instance returned a value.
    pub fn is_returned(&self) -> bool {
        self.returned_ref().is_ok()
    }

    /// Unwraps a run result into a return value.
    ///
    /// # Panics
    ///
    /// Panics if the instance instead yielded, with a panic message including the passed message.
    pub fn expect_returned(self, msg: &str) -> UntypedRetVal {
        self.returned().expect(msg)
    }

    /// Unwraps a run result into a returned value.
    ///
    /// # Panics
    ///
    /// Panics if the instance instead yielded.
    pub fn unwrap_returned(self) -> UntypedRetVal {
        self.returned().unwrap()
    }

    /// Try to get a yielded value from a run result, returning `Error::InstanceNotYielded` if the
    /// instance instead returned.
    pub fn yielded(self) -> Result<YieldedVal, Error> {
        match self {
            RunResult::Returned(_) => Err(Error::InstanceNotYielded),
            RunResult::Yielded(yv) => Ok(yv),
        }
    }

    /// Try to get a reference to a yielded value from a run result, returning
    /// `Error::InstanceNotYielded` if the instance instead returned.
    pub fn yielded_ref(&self) -> Result<&YieldedVal, Error> {
        match self {
            RunResult::Returned(_) => Err(Error::InstanceNotYielded),
            RunResult::Yielded(yv) => Ok(yv),
        }
    }

    /// Returns `true` if the instance yielded.
    pub fn is_yielded(&self) -> bool {
        self.yielded_ref().is_ok()
    }

    /// Returns `true` if the instance can be resumed: either it has yielded, or its bound has
    /// expired.
    pub fn can_resume(&self) -> bool {
        self.is_yielded()
    }

    /// Returns `true` if the instance has yielded a value of the given type.
    pub fn has_yielded<A: Any>(&self) -> bool {
        match self {
            RunResult::Yielded(yv) => yv.is::<A>(),
            _ => false,
        }
    }

    /// Unwraps a run result into a yielded value.
    ///
    /// # Panics
    ///
    /// Panics if the instance instead returned, with a panic message including the passed message.
    pub fn expect_yielded(self, msg: &str) -> YieldedVal {
        self.yielded().expect(msg)
    }

    /// Unwraps a run result into a yielded value.
    ///
    /// # Panics
    ///
    /// Panics if the instance instead returned.
    pub fn unwrap_yielded(self) -> YieldedVal {
        self.yielded().unwrap()
    }
}

/// An "internal" run result: either a `RunResult` or a bound expiration. We do not expose bound
/// expirations to the caller directly; rather, we only handle them in `run_async()`.
pub(crate) enum InternalRunResult {
    Normal(RunResult),
    BoundExpired,
}

impl InternalRunResult {
    pub(crate) fn unwrap(self) -> RunResult {
        match self {
            InternalRunResult::Normal(result) => result,
            InternalRunResult::BoundExpired => panic!("should not have had a runtime bound"),
        }
    }
}

impl std::convert::Into<InternalRunResult> for RunResult {
    fn into(self) -> InternalRunResult {
        InternalRunResult::Normal(self)
    }
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
    /// let retval = instance.run("factorial", &[5u64.into()]).unwrap().unwrap_returned();
    /// assert_eq!(u64::from(retval), 120u64);
    ///
    /// // runtime faults yield `Err(Error)`
    /// let result = instance.run("faulting_function", &[]);
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
    pub fn run(&mut self, entrypoint: &str, args: &[Val]) -> Result<RunResult, Error> {
        let func = self.module.get_export_func(entrypoint)?;
        Ok(self.run_func(func, &args, false, None)?.unwrap())
    }

    /// Run a function with arguments in the guest context from the [WebAssembly function
    /// table](https://webassembly.github.io/spec/core/syntax/modules.html#tables).
    ///
    /// # Safety
    ///
    /// The same safety caveats of [`Instance::run()`](struct.Instance.html#method.run) apply.
    pub fn run_func_idx(
        &mut self,
        table_idx: u32,
        func_idx: u32,
        args: &[Val],
    ) -> Result<RunResult, Error> {
        let func = self.module.get_func_from_idx(table_idx, func_idx)?;
        Ok(self.run_func(func, &args, false, None)?.unwrap())
    }

    /// Resume execution of an instance that has yielded without providing a value to the guest.
    ///
    /// This should only be used when the guest yielded with
    /// [`Vmctx::yield_()`](vmctx/struct.Vmctx.html#method.yield_) or
    /// [`Vmctx::yield_val()`](vmctx/struct.Vmctx.html#method.yield_val).
    ///
    /// # Safety
    ///
    /// The foreign code safety caveat of [`Instance::run()`](struct.Instance.html#method.run)
    /// applies.
    pub fn resume(&mut self) -> Result<RunResult, Error> {
        self.resume_with_val(EmptyYieldVal)
    }

    /// Resume execution of an instance that has yielded, providing a value to the guest.
    ///
    /// The type of the provided value must match the type expected by
    /// [`Vmctx::yield_expecting_val()`](vmctx/struct.Vmctx.html#method.yield_expecting_val) or
    /// [`Vmctx::yield_val_expecting_val()`](vmctx/struct.Vmctx.html#method.yield_val_expecting_val).
    ///
    /// The provided value will be dynamically typechecked against the type the guest expects to
    /// receive, and if that check fails, this call will fail with `Error::InvalidArgument`.
    ///
    /// # Safety
    ///
    /// The foreign code safety caveat of [`Instance::run()`](struct.Instance.html#method.run)
    /// applies.
    pub fn resume_with_val<A: Any + 'static>(&mut self, val: A) -> Result<RunResult, Error> {
        Ok(self.resume_with_val_impl(val, false, None)?.unwrap())
    }

    pub(crate) fn resume_with_val_impl<A: Any + 'static>(
        &mut self,
        val: A,
        async_context: bool,
        max_insn_count: Option<u64>,
    ) -> Result<InternalRunResult, Error> {
        match &self.state {
            State::Yielded { expecting, .. } => {
                // make sure the resumed value is of the right type
                if !expecting.is::<PhantomData<A>>() {
                    return Err(Error::InvalidArgument(
                        "type mismatch between yielded instance expected value and resumed value",
                    ));
                }
            }
            _ => return Err(Error::InvalidArgument("can only resume a yielded instance")),
        }

        self.resumed_val = Some(Box::new(val) as Box<dyn Any + 'static>);

        self.set_instruction_bound_delta(max_insn_count);
        self.swap_and_return(async_context)
    }

    /// Resume execution of an instance that has previously reached an instruction bound.
    ///
    /// The execution slice that begins with this call is bounded by the new bound provided.
    ///
    /// This should only be used when `run_func()` returned a `RunResult::Bounded`. This is an
    /// internal function used by `run_async()`.
    ///
    /// # Safety
    ///
    /// The foreign code safety caveat of [`Instance::run()`](struct.Instance.html#method.run)
    /// applies.
    pub(crate) fn resume_bounded(
        &mut self,
        max_insn_count: u64,
    ) -> Result<InternalRunResult, Error> {
        if !self.state.is_bound_expired() {
            return Err(Error::InvalidArgument(
                "can only call resume_bounded() on an instance that hit an instruction bound",
            ));
        }
        self.set_instruction_bound_delta(Some(max_insn_count));
        self.swap_and_return(true)
    }

    /// Run the module's [start function][start], if one exists.
    ///
    /// If there is no start function in the module, this does nothing.
    ///
    /// If the module contains a start function, you must run it before running any other exported
    /// functions. If an instance is reset, you must run the start function again.
    ///
    /// Start functions may assume that Wasm tables and memories are properly initialized, but may
    /// not assume that imported functions or globals are available.
    ///
    /// # Errors
    ///
    /// In addition to the errors that can be returned from [`Instance::run()`][run], this can also
    /// return `Error::StartYielded` if the start function attempts to yield. This should not arise
    /// as long as the start function does not attempt to use any imported functions.
    ///
    /// This also returns `Error::StartAlreadyRun` if the start function has already run since the
    /// instance was created or last reset.
    ///
    /// Wasm start functions are not allowed to call imported functions. If the start function
    /// attempts to do so, the instance will be terminated with
    /// `TerminationDetails::StartCalledImportFunc`.
    ///
    /// # Safety
    ///
    /// The foreign code safety caveat of [`Instance::run()`][run]
    /// applies.
    ///
    /// [run]: struct.Instance.html#method.run
    /// [start]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-start
    pub fn run_start(&mut self) -> Result<(), Error> {
        if let Some(start) = self.module.get_start_func()? {
            if !self.is_not_started() {
                return Err(Error::StartAlreadyRun);
            }
            self.run_func(start, &[], false, None)?;
        }
        Ok(())
    }

    /// Reset the instance's heap and global variables to their initial state.
    ///
    /// The WebAssembly `start` section, if present, will need to be re-run with
    /// [`Instance::run_start()`][run_start] before running any other exported functions.
    ///
    /// The embedder contexts present at instance creation or added with
    /// [`Instance::insert_embed_ctx()`](struct.Instance.html#method.insert_embed_ctx) are not
    /// modified by this call; it is the embedder's responsibility to clear or reset their state if
    /// necessary.
    ///
    /// This will also reinitialize the kill state, which means that any outstanding
    /// [`KillSwitch`](struct.KillSwitch.html) objects will be unable to terminate this instance.
    /// It is the embedder's responsibility to initialize new `KillSwitch`es after resetting an
    /// instance.
    ///
    /// [run_start]: struct.Instance.html#method.run
    pub fn reset(&mut self) -> Result<(), Error> {
        self.alloc.reset_heap(self.module.as_ref())?;
        let globals = unsafe { self.alloc.globals_mut() };
        let mod_globals = self.module.globals();
        for (i, v) in mod_globals.iter().enumerate() {
            globals[i] = match v.global() {
                Global::Import { .. } => {
                    return Err(Error::Unsupported(format!(
                        "global imports are unsupported; found: {:?}",
                        v
                    )));
                }
                Global::Def(def) => def.init_val(),
            };
        }

        if self.module.get_start_func()?.is_some() {
            self.state = State::NotStarted;
        } else {
            self.state = State::Ready;
        }

        #[cfg(feature = "concurrent_testpoints")]
        {
            self.kill_state = Arc::new(KillState::new(Arc::clone(&self.lock_testpoints)));
        }
        #[cfg(not(feature = "concurrent_testpoints"))]
        {
            self.kill_state = Arc::new(KillState::new());
        }

        Ok(())
    }

    /// Grow the guest memory by the given number of WebAssembly pages.
    ///
    /// On success, returns the number of pages that existed before the call.
    pub fn grow_memory(&mut self, additional_pages: u32) -> Result<u32, Error> {
        let additional_bytes = additional_pages
            .checked_mul(WASM_PAGE_SIZE)
            .ok_or_else(|| lucet_format_err!("additional pages larger than wasm address space",))?;
        let orig_len = self
            .alloc
            .expand_heap(additional_bytes, self.module.as_ref())?;
        Ok(orig_len / WASM_PAGE_SIZE)
    }

    /// Grow memory from a hostcall context.
    pub fn grow_memory_from_hostcall(
        &mut self,
        vmctx: &Vmctx,
        additional_pages: u32,
    ) -> Result<u32, Error> {
        // Use a function so that we can report all Errs via memory_grow_failed.
        fn aux(
            instance: &mut Instance,
            vmctx: &Vmctx,
            additional_pages: u32,
        ) -> Result<u32, Error> {
            // Calculate current and desired bytes
            let current_bytes = instance.alloc.heap_len();
            let additional_bytes =
                additional_pages
                    .checked_mul(WASM_PAGE_SIZE)
                    .ok_or_else(|| {
                        lucet_format_err!("additional pages larger than wasm address space",)
                    })? as usize;
            let desired_bytes = additional_bytes
                .checked_add(current_bytes)
                .ok_or_else(|| lucet_format_err!("desired bytes overflow",))?;
            // Let the limiter reject the grow
            if let Some(ref mut limiter) = instance.memory_limiter {
                if !vmctx.block_on(async move {
                    limiter.memory_growing(current_bytes, desired_bytes).await
                }) {
                    lucet_bail!("memory limiter denied growth");
                }
            }
            // Try the grow itself
            instance.grow_memory(additional_pages)
        }

        match aux(self, vmctx, additional_pages) {
            Ok(n) => Ok(n),
            Err(e) => {
                if let Some(ref mut limiter) = self.memory_limiter {
                    limiter.memory_grow_failed(&e);
                    Err(e)
                } else {
                    Err(e)
                }
            }
        }
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
    pub fn globals(&self) -> &[GlobalValue] {
        unsafe { self.alloc.globals() }
    }

    /// Return the WebAssembly globals as a mutable slice of `i64`s.
    pub fn globals_mut(&mut self) -> &mut [GlobalValue] {
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
    pub fn get_embed_ctx<T: Any>(&self) -> Option<Result<Ref<'_, T>, BorrowError>> {
        self.embed_ctx.try_get::<T>()
    }

    /// Get a mutable reference to a context value of a particular type, if it exists.
    pub fn get_embed_ctx_mut<T: Any>(&self) -> Option<Result<RefMut<'_, T>, BorrowMutError>> {
        self.embed_ctx.try_get_mut::<T>()
    }

    /// Insert a context value.
    ///
    /// If a context value of the same type already existed, it is returned.
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
            + Fn(
                &Instance,
                &Option<TrapCode>,
                libc::c_int,
                *const siginfo_t,
                *const c_void,
            ) -> SignalBehavior,
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

    /// Set whether the Lucet signal handler is installed when running or resuming this instance
    /// (`true` by default).
    ///
    /// If this is `true`, the Lucet runtime checks whether its signal handler is installed whenever
    /// an instance runs, installing it if it is not present, and uninstalling it when there are no
    /// longer any Lucet instances running. If this is `false`, that check is disabled, which can
    /// improve performance when running or resuming an instance.
    ///
    /// Use `install_lucet_signal_handler()` and `remove_lucet_signal_handler()` to manually install
    /// or remove the signal handler.
    ///
    /// # Safety
    ///
    /// If the Lucet signal handler is not installed when an instance runs, WebAssembly traps such
    /// as division by zero, assertion failures, or out-of-bounds memory access will raise signals
    /// to the default signal handlers, usually causing the entire process to crash.
    pub fn ensure_signal_handler_installed(&mut self, ensure: bool) {
        self.ensure_signal_handler_installed = ensure;
    }

    /// Set whether an alternate signal stack is installed for the current thread when running or
    /// resuming this instance (`true` by default).
    ///
    /// If this is `true`, the Lucet runtime installs an alternate signal stack whenever an instance
    /// runs, and uninstalls it afterwards. If this is `false`, the signal stack is not
    /// automatically manipulated.
    ///
    /// The automatically-installed signal stack uses space allocated in the instance's `Region`,
    /// sized according to the `signal_stack_size` field of the region's `Limits`.
    ///
    /// If you wish to instead provide your own signal stack, we recommend using a stack of size
    /// `DEFAULT_SIGNAL_STACK_SIZE`, which varies depending on platform and optimization level.
    ///
    /// Signal stacks are installed on a per-thread basis, so any thread that runs this instance
    /// must have a signal stack installed.
    ///
    /// # Safety
    ///
    /// If an alternate signal stack is not installed when an instance runs, there may not be enough
    /// stack space for the Lucet signal handler to run. If the signal handler runs out of stack
    /// space, a double fault could occur and crash the entire process, or the program could
    /// continue with corrupted memory.
    pub fn ensure_sigstack_installed(&mut self, ensure: bool) {
        self.ensure_sigstack_installed = ensure;
    }

    pub fn kill_switch(&self) -> KillSwitch {
        KillSwitch::new(Arc::downgrade(&self.kill_state))
    }

    pub fn is_not_started(&self) -> bool {
        self.state.is_not_started()
    }

    pub fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    pub fn is_yielded(&self) -> bool {
        self.state.is_yielded()
    }

    pub fn is_bound_expired(&self) -> bool {
        self.state.is_bound_expired()
    }

    pub fn is_faulted(&self) -> bool {
        self.state.is_faulted()
    }

    pub fn is_terminated(&self) -> bool {
        self.state.is_terminated()
    }

    // This needs to be public as it's used in the expansion of `lucet_hostcalls`, available for
    // external use. But you *really* shouldn't have to call this yourself, so we're going to keep
    // it out of rustdoc.
    #[doc(hidden)]
    pub fn uninterruptable<T, F: FnOnce() -> T>(&mut self, f: F) -> T {
        self.kill_state.begin_hostcall();
        let res = f();
        let stop_reason = self.kill_state.end_hostcall();

        if let Some(termination_details) = stop_reason {
            // TODO: once we have unwinding, panic here instead so we unwind host frames
            unsafe {
                self.terminate(termination_details);
            }
        }

        res
    }

    #[inline]
    pub fn get_instruction_count(&self) -> Option<u64> {
        if self.module.is_instruction_count_instrumented() {
            let implicits = self.get_instance_implicits();
            let sum = implicits.instruction_count_bound + implicits.instruction_count_adj;
            // This invariant is ensured as we always set up the fields to have a positive sum, and
            // generated code only increments `adj`.
            debug_assert!(sum >= 0);
            return Some(sum as u64);
        }
        None
    }

    /// Set the total instruction count and bound.
    #[inline]
    pub fn set_instruction_count_and_bound(&mut self, instruction_count: u64, bound: u64) {
        let implicits = self.get_instance_implicits_mut();
        let instruction_count =
            i64::try_from(instruction_count).expect("instruction count too large");
        let bound = i64::try_from(bound).expect("bound too large");
        // These two sum to `instruction_count`, which must be non-negative.
        implicits.instruction_count_bound = bound;
        implicits.instruction_count_adj = instruction_count - bound;
    }

    /// Set the instruction bound to be `delta` above the current count.
    ///
    /// See the comments on `instruction_count_adj` in `InstanceRuntimeData` for more details on
    /// how this bound works; most relevant is that a bound-yield is only triggered if the bound
    /// value is *crossed*, but not if execution *begins* with the value exceeded. Hence `delta`
    /// must be greater than zero for this to set up the instance state to trigger a yield.
    #[inline]
    pub fn set_instruction_bound_delta(&mut self, delta: Option<u64>) {
        let implicits = self.get_instance_implicits_mut();
        let sum = implicits.instruction_count_adj + implicits.instruction_count_bound;
        let delta = delta.unwrap_or(i64::MAX as u64);
        let delta = i64::try_from(delta).expect("delta too large");
        implicits.instruction_count_bound = sum.wrapping_add(delta);
        implicits.instruction_count_adj = -delta;
    }

    #[inline]
    pub fn set_hostcall_stack_reservation(&mut self) {
        let slot = self
            .alloc
            .slot
            .as_ref()
            .expect("reachable instance has a slot");

        let reservation = slot.limits.hostcall_reservation;

        // The `.stack` field is a pointer to the lowest address of the stack - the start of its
        // allocation. Because the stack grows downward, this is the end of the stack space. So the
        // limit we'll need to check for hostcalls is some reserved space upwards from here, to
        // meet some guest stack pointer early.
        self.get_instance_implicits_mut().stack_limit = slot.stack as u64 + reservation as u64;
    }

    /// Set a memory limiter for the instance.
    ///
    /// If set, this instance must be run asynchronously via [`InstanceHandle::run_async`]
    pub fn set_memory_limiter(&mut self, limiter: Box<dyn MemoryLimiter + Send + Sync + 'static>) {
        self.memory_limiter = Some(limiter)
    }
}

// Private API
impl Instance {
    fn new(alloc: Alloc, module: Arc<dyn Module>, embed_ctx: CtxMap) -> Self {
        let globals_ptr = alloc.slot().globals as *mut i64;

        #[cfg(feature = "concurrent_testpoints")]
        let lock_testpoints = Arc::new(LockTestpoints::new());

        #[cfg(feature = "concurrent_testpoints")]
        let kill_state = Arc::new(KillState::new(Arc::clone(&lock_testpoints)));
        #[cfg(not(feature = "concurrent_testpoints"))]
        let kill_state = Arc::new(KillState::new());

        let mut inst = Instance {
            magic: LUCET_INSTANCE_MAGIC,
            embed_ctx,
            module,
            ctx: Context::new(),
            state: State::Ready,
            kill_state,
            #[cfg(feature = "concurrent_testpoints")]
            lock_testpoints,
            alloc: ManuallyDrop::new(alloc),
            fatal_handler: default_fatal_handler,
            c_fatal_handler: None,
            signal_handler: Box::new(signal_handler_none) as Box<SignalHandler>,
            ensure_signal_handler_installed: true,
            ensure_sigstack_installed: true,
            entrypoint: None,
            resumed_val: None,
            memory_limiter: None,
            _padding: (),
        };
        inst.set_globals_ptr(globals_ptr);
        inst.set_instruction_count_and_bound(0, 0);
        // Ensure the hostcall limit tracked in this instance's guest-shared data is up-to-date.
        inst.set_hostcall_stack_reservation();

        assert_eq!(mem::size_of::<Instance>(), HOST_PAGE_SIZE_EXPECTED);
        let unpadded_size = offset_of!(Instance, _padding);
        assert!(unpadded_size <= HOST_PAGE_SIZE_EXPECTED - mem::size_of::<*mut i64>());
        inst
    }

    // The globals pointer must be stored right before the end of the structure, padded to the page size,
    // so that it is 8 bytes before the heap.
    // For this reason, the alignment of the structure is set to 4096, and we define accessors that
    // read/write the globals pointer as bytes [4096-8..4096] of that structure represented as raw bytes.
    // InstanceRuntimeData is placed such that it ends at the end of the page this `Instance` starts
    // on. So we can access it by *self + PAGE_SIZE - size_of::<InstanceRuntimeData>
    #[inline]
    fn get_instance_implicits(&self) -> &InstanceRuntimeData {
        unsafe {
            let implicits_ptr = (self as *const _ as *const u8)
                .add(HOST_PAGE_SIZE_EXPECTED - mem::size_of::<InstanceRuntimeData>())
                as *const InstanceRuntimeData;
            mem::transmute::<*const InstanceRuntimeData, &InstanceRuntimeData>(implicits_ptr)
        }
    }

    #[inline]
    fn get_instance_implicits_mut(&mut self) -> &mut InstanceRuntimeData {
        unsafe {
            let implicits_ptr = (self as *mut _ as *mut u8)
                .add(HOST_PAGE_SIZE_EXPECTED - mem::size_of::<InstanceRuntimeData>())
                as *mut InstanceRuntimeData;
            mem::transmute::<*mut InstanceRuntimeData, &mut InstanceRuntimeData>(implicits_ptr)
        }
    }

    #[allow(dead_code)]
    #[inline]
    fn get_globals_ptr(&self) -> *mut i64 {
        self.get_instance_implicits().globals_ptr
    }

    #[inline]
    fn set_globals_ptr(&mut self, globals_ptr: *mut i64) {
        self.get_instance_implicits_mut().globals_ptr = globals_ptr
    }

    /// Run a function in guest context at the given entrypoint.
    pub(crate) fn run_func(
        &mut self,
        func: FunctionHandle,
        args: &[Val],
        async_context: bool,
        inst_count_bound: Option<u64>,
    ) -> Result<InternalRunResult, Error> {
        let needs_start = self.state.is_not_started() && !func.is_start_func;
        if needs_start {
            return Err(Error::InstanceNeedsStart);
        }

        let is_ready = self.state.is_ready();
        let is_starting = self.state.is_not_started() && func.is_start_func;
        let is_non_fatally_faulted = self.state.is_faulted() && !self.state.is_fatal();
        if !(is_ready || is_starting || is_non_fatally_faulted) {
            return Err(Error::InvalidArgument(
                "instance must be ready, starting, or non-fatally faulted",
            ));
        }
        if func.ptr.as_usize() == 0 {
            return Err(Error::InvalidArgument(
                "entrypoint function cannot be null; this is probably a malformed module",
            ));
        }

        let sig = self.module.get_signature(func.id);

        // in typechecking these values, we can only really check that arguments are correct.
        // in the future we might want to make return value use more type safe as well.

        if sig.params.len() != args.len() {
            return Err(Error::InvalidArgument(
                "entrypoint function signature mismatch (number of arguments is incorrect)",
            ));
        }

        for (param_ty, arg) in sig.params.iter().zip(args.iter()) {
            if param_ty != &arg.value_type() {
                return Err(Error::InvalidArgument(
                    "entrypoint function signature mismatch",
                ));
            }
        }

        self.entrypoint = Some(func);

        let mut args_with_vmctx = vec![Val::from(self.alloc.slot().heap)];
        args_with_vmctx.extend_from_slice(args);

        self.set_instruction_bound_delta(inst_count_bound);

        let self_ptr = self as *mut _;
        Context::init_with_callback(
            unsafe { self.alloc.stack_u64_mut() },
            &mut self.ctx,
            execution::exit_guest_region,
            self_ptr,
            func.ptr.as_usize(),
            &args_with_vmctx,
        )?;

        self.install_activator();
        self.swap_and_return(async_context)
    }

    /// Prepare the guest so that it will update its execution domain upon entry.
    ///
    /// This mutates the context's registers so that an activation function that will be run after
    /// performing a context switch. This function (`enter_guest_region`) will mark the guest as
    /// terminable before continuing to whatever guest code we want to run.
    ///
    /// `lucet_context_activate` takes three arguments in the following registers:
    ///   * rdi: the data for the entry callback.
    ///   * rsi: the address of the entry callback.
    ///   * rbx: the address of the guest code to execute.
    ///
    /// The appropriate value for `rbx` is the top of the guest stack, which we would otherwise
    /// return to and start executing immediately. For `rdi`, we want to pass our callback data
    /// (a raw pointer to the instance). This will be passed as the first argument to the entry
    /// function, which is responsible for updating the kill state's execution domain.
    ///
    /// See `lucet_runtime_internals::context::lucet_context_activate`, and
    /// `execution::enter_guest_region` for more info.
    // TODO KTM 2020-03-13: This should be a method on `Context`.
    fn install_activator(&mut self) {
        unsafe {
            // Get a raw pointer to the top of the guest stack.
            let top_of_stack = self.ctx.gpr.rsp as *mut u64;
            // Move the guest code address to rbx, and then put the address of the activation thunk
            // at the top of the stack, so that we will start execution at `enter_guest_region`.
            self.ctx.gpr.rbx = *top_of_stack;
            *top_of_stack = crate::context::lucet_context_activate as u64;
            // Pass a pointer to our guest-side entrypoint bootstrap code in `rsi`, and then put
            // its first argument (a raw pointer to `self`) in `rdi`.
            self.ctx.gpr.rsi = execution::enter_guest_region as u64;
            self.ctx.gpr.rdi = self.ctx.callback_data_ptr() as u64;
        }
    }

    /// The core routine for context switching into a guest, and extracting a result.
    ///
    /// This must only be called for an instance in a ready, non-fatally faulted, or yielded state,
    /// or in the not-started state on the start function. The public wrappers around this function
    /// should make sure the state is appropriate.
    fn swap_and_return(&mut self, async_context: bool) -> Result<InternalRunResult, Error> {
        let is_start_func = self
            .entrypoint
            .expect("we always have an entrypoint by now")
            .is_start_func;
        debug_assert!(
            self.state.is_ready()
                || self.state.is_not_started() && is_start_func
                || (self.state.is_faulted() && !self.state.is_fatal())
                || self.state.is_yielded()
                || self.state.is_bound_expired()
        );
        self.state = State::Running { async_context };

        let res = self.with_current_instance(|i| {
            i.with_signals_on(|i| {
                HOST_CTX.with(|host_ctx| {
                    // Save the current context into `host_ctx`, and jump to the guest context. The
                    // lucet context is linked to host_ctx, so it will return here after it finishes,
                    // successfully or otherwise.
                    unsafe { Context::swap(&mut *host_ctx.get(), &mut i.ctx) };
                    Ok(())
                })
            })
        });

        #[cfg(feature = "concurrent_testpoints")]
        self.lock_testpoints
            .instance_after_clearing_current_instance
            .check();

        if let Err(e) = res {
            // Something went wrong setting up or tearing down the signal handlers and signal
            // stack. This is an error, but we don't want it to mask an error that may have arisen
            // due to a guest fault or guest termination. So, we set the state back to `Ready` or
            // `NotStarted` only if it is still `Running`, which likely indicates we never even made
            // it into the guest.
            //
            // As of 2020-03-20, the only early return points in the code above happen before the
            // guest would be able to run, so this should always transition from running to
            // ready or not started if there's an error.
            if let State::Running { .. } = self.state {
                if is_start_func {
                    self.state = State::NotStarted;
                } else {
                    self.state = State::Ready;
                }
            }
            return Err(e);
        }

        // Sandbox has jumped back to the host process, indicating it has either:
        //
        // * returned: state should be `Running`; transition to `Ready` and return a RunResult
        // * yielded: state should be `Yielding`; transition to `Yielded` and return a RunResult
        // * trapped: state should be `Faulted`; populate details and return an error or call a handler as appropriate
        // * terminated: state should be `Terminating`; transition to `Terminated` and return the termination details as an Err
        //
        // The state should never be `Ready`, `Terminated`, `Yielded`, or `Transitioning` at this point

        // Set transitioning state temporarily so that we can move values out of the current state
        let st = mem::replace(&mut self.state, State::Transitioning);

        if !st.is_yielding() && !st.is_bound_expired() {
            // If the instance is *not* yielding, initialize a fresh `KillState` for subsequent
            // executions, which will invalidate any existing `KillSwitch`'s weak references.
            #[cfg(feature = "concurrent_testpoints")]
            {
                self.kill_state = Arc::new(KillState::new(Arc::clone(&self.lock_testpoints)));
            }
            #[cfg(not(feature = "concurrent_testpoints"))]
            {
                self.kill_state = Arc::new(KillState::default());
            }
        }

        match st {
            State::Running { .. } => {
                let retval = self.ctx.get_untyped_retval();
                self.state = State::Ready;
                Ok(RunResult::Returned(retval).into())
            }
            State::Terminating { details, .. } => {
                self.state = State::Terminated;
                Err(Error::RuntimeTerminated(details).into())
            }
            State::Yielding { val, expecting } => {
                self.state = State::Yielded { expecting };
                Ok(RunResult::Yielded(val).into())
            }
            State::Faulted {
                mut details,
                siginfo,
                context,
            } => {
                // Sandbox is no longer runnable. It's unsafe to determine all error details in the signal
                // handler, so we fill in extra details here.
                //
                // FIXME after lucet-module is complete it should be possible to fill this in without
                // consulting the process symbol table
                details.rip_addr_details = self
                    .module
                    .addr_details(details.rip_addr as *const c_void)?;

                // fill the state back in with the updated details in case fatal handlers need it
                self.state = State::Faulted {
                    details: details.clone(),
                    siginfo,
                    context,
                };

                if details.fatal {
                    // Some errors indicate that the guest is not functioning correctly or that
                    // the loaded code violated some assumption, so bail out via the fatal
                    // handler.

                    // Run the C-style fatal handler, if it exists.
                    if let Some(h) = self.c_fatal_handler {
                        unsafe { h(self as *mut Instance) }
                    }

                    // If there is no C-style fatal handler, or if it (erroneously) returns,
                    // call the Rust handler that we know will not return
                    (self.fatal_handler)(self)
                } else {
                    // leave the full fault details in the instance state, and return the
                    // higher-level info to the user
                    Err(Error::RuntimeFault(details).into())
                }
            }
            State::BoundExpired => {
                self.state = State::BoundExpired;
                Ok(InternalRunResult::BoundExpired)
            }
            State::NotStarted
            | State::Ready
            | State::Terminated
            | State::Yielded { .. }
            | State::Transitioning => Err(lucet_format_err!(
                "\"impossible\" state found in `swap_and_return()`: {}",
                st
            )),
        }
    }

    fn with_current_instance<F, R>(&mut self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&mut Instance) -> Result<R, Error>,
    {
        CURRENT_INSTANCE.with(|current_instance| {
            let mut current_instance = current_instance.borrow_mut();
            lucet_ensure!(
                current_instance.is_none(),
                "no instance must already be running on this thread"
            ); // safety: `self` is not null if we are in this function
            *current_instance = Some(unsafe { NonNull::new_unchecked(self) });
            Ok(())
        })?;

        self.kill_state.schedule(unsafe { pthread_self() });

        let res = f(self);

        self.kill_state.deschedule();

        CURRENT_INSTANCE.with(|current_instance| {
            *current_instance.borrow_mut() = None;
        });

        res
    }
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
    pub trapcode: Option<TrapCode>,
    /// The instruction pointer where the fault occurred.
    pub rip_addr: uintptr_t,
    /// Extra information about the instruction pointer's location, if available.
    pub rip_addr_details: Option<module::AddrDetails>,
}

impl std::fmt::Display for FaultDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fatal {
            write!(f, "fault FATAL ")?;
        } else {
            write!(f, "fault ")?;
        }

        if let Some(trapcode) = self.trapcode {
            write!(f, "{:?} ", trapcode)?;
        } else {
            write!(f, "TrapCode::UNKNOWN ")?;
        }

        write!(f, "code at address {:p}", self.rip_addr as *const c_void)?;

        if let Some(ref addr_details) = self.rip_addr_details {
            if let Some(ref fname) = addr_details.file_name {
                let sname = addr_details.sym_name.as_deref().unwrap_or("<unknown>");
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
pub enum TerminationDetails {
    /// Returned when a signal handler terminates the instance.
    Signal,
    /// Returned when `get_embed_ctx` or `get_embed_ctx_mut` are used with a type that is not present.
    CtxNotFound,
    /// Returned when the type of the value passed to `Instance::resume_with_val()` does not match
    /// the type expected by `Vmctx::yield_expecting_val()` or `Vmctx::yield_val_expecting_val`, or
    /// if `Instance::resume()` was called when a value was expected.
    ///
    /// **Note**: If you see this termination value, please report it as a Lucet bug. The types of
    /// resumed values are dynamically checked by `Instance::resume()` and
    /// `Instance::resume_with_val()`, so this should never arise.
    YieldTypeMismatch,
    /// Returned when dynamic borrowing rules of methods like `Vmctx::heap()` are violated.
    BorrowError(&'static str),
    /// Calls to `lucet_hostcall_terminate` provide a payload for use by the embedder.
    Provided {
        type_name: &'static str,
        provided: Box<dyn Any + 'static>,
    },
    /// The instance was terminated by its `KillSwitch`.
    Remote,
    /// A panic occurred during a hostcall other than the specialized panic used to implement
    /// Lucet runtime features.
    ///
    /// Panics are raised by the Lucet runtime in order to unwind the hostcall before jumping back
    /// to the host context for any of the reasons described by the variants of this type. The panic
    /// payload in that case is a already a `TerminationDetails` value.
    ///
    /// This variant is created when any type other than `TerminationDetails` is the payload of a
    /// panic arising during a hostcall, meaning it was not intentionally raised by the Lucet
    /// runtime.
    ///
    /// The panic payload contained in this variant should be rethrown using
    /// [`resume_unwind`](https://doc.rust-lang.org/std/panic/fn.resume_unwind.html) once returned
    /// to the host context.
    ///
    /// Note that this variant will be removed once cross-FFI unwinding support lands in
    /// [Rust](https://github.com/rust-lang/rfcs/pull/2945) and
    /// [Lucet](https://github.com/bytecodealliance/lucet/pull/254).
    OtherPanic(Box<dyn Any + Send + 'static>),
    /// The instance was terminated by `Vmctx::block_on` being called from an instance
    /// that isnt running in an async context
    BlockOnNeedsAsync,
}

impl TerminationDetails {
    pub fn provide<A: Any + 'static>(details: A) -> Self {
        TerminationDetails::Provided {
            type_name: std::any::type_name::<A>(),
            provided: Box::new(details),
        }
    }
    pub fn provided_details(&self) -> Option<&dyn Any> {
        match self {
            TerminationDetails::Provided { provided, .. } => Some(provided.as_ref()),
            _ => None,
        }
    }
    /// Try to interpret the termination details as a provided exit code.
    ///
    /// The most consistent form of `TerminationDetails::Provided` comes from Lucet's
    /// implementation of `proc_exit`, which exits with a `Provided` holding the given exit code.
    /// For cases where a Lucet user simply wants "`proc_exit` or continue panicking" behavior,
    /// `as_exitcode` can simplify handling `TerminationDetails`.
    pub fn as_exitcode(&self) -> Option<u32> {
        match self {
            TerminationDetails::Provided { provided, .. } => {
                // I apologize for this load-bearing `as u32`.
                // Wasi uses an u32 for the proc_exist status (`lucet_wasi::Exitcode`) in the
                // witx. However, wasmtime::Trap exit status is an i32, so the
                // wiggle::Trap::I32Exit variant mirrors Wasmtime. The `as u32` lets this method
                // return a type equivalent to `lucet_wasi::Exitcode`, but users interested in the
                // full range of `wiggle::Trap` will have to handle an i32 variant.
                match provided.downcast_ref::<wiggle::Trap>() {
                    Some(wiggle::Trap::I32Exit(code)) => Some(*code as u32),
                    _ => None,
                }
            }
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

impl PartialEq for TerminationDetails {
    fn eq(&self, rhs: &TerminationDetails) -> bool {
        use TerminationDetails::*;
        match (self, rhs) {
            (Signal, Signal) => true,
            (BorrowError(msg1), BorrowError(msg2)) => msg1 == msg2,
            (CtxNotFound, CtxNotFound) => true,
            (BlockOnNeedsAsync, BlockOnNeedsAsync) => true,
            // can't compare `Any`
            _ => false,
        }
    }
}

impl std::fmt::Debug for TerminationDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TerminationDetails::")?;
        match self {
            TerminationDetails::Signal => write!(f, "Signal"),
            TerminationDetails::BorrowError(msg) => write!(f, "BorrowError({})", msg),
            TerminationDetails::CtxNotFound => write!(f, "CtxNotFound"),
            TerminationDetails::YieldTypeMismatch => write!(f, "YieldTypeMismatch"),
            TerminationDetails::Provided { type_name, .. } => write!(f, "Provided({})", type_name),
            TerminationDetails::Remote => write!(f, "Remote"),
            TerminationDetails::OtherPanic(_) => write!(f, "OtherPanic(Any)"),
            TerminationDetails::BlockOnNeedsAsync => write!(f, "BlockOnNeedsAsync"),
        }
    }
}

unsafe impl Send for TerminationDetails {}
unsafe impl Sync for TerminationDetails {}

/// The value yielded by an instance through a [`Vmctx`](vmctx/struct.Vmctx.html) and returned to
/// the host.
pub struct YieldedVal {
    val: Box<dyn Any + Send + 'static>,
}

impl std::fmt::Debug for YieldedVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            write!(f, "YieldedVal {{ val: None }}")
        } else {
            write!(f, "YieldedVal {{ val: Some }}")
        }
    }
}

impl YieldedVal {
    pub(crate) fn new<A: Any + Send + 'static>(val: A) -> Self {
        YieldedVal { val: Box::new(val) }
    }

    /// Returns `true` if the guest yielded the parameterized type.
    pub fn is<A: Any>(&self) -> bool {
        self.val.is::<A>()
    }

    /// Returns `true` if the guest yielded without a value.
    pub fn is_none(&self) -> bool {
        self.is::<EmptyYieldVal>()
    }

    /// Returns `true` if the guest yielded with a value.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Attempt to downcast the yielded value to a concrete type, returning the original
    /// `YieldedVal` if unsuccessful.
    pub fn downcast<A: Any + Send + 'static>(self) -> Result<Box<A>, YieldedVal> {
        match self.val.downcast() {
            Ok(val) => Ok(val),
            Err(val) => Err(YieldedVal { val }),
        }
    }

    /// Returns a reference to the yielded value if it is present and of type `A`, or `None` if it
    /// isn't.
    pub fn downcast_ref<A: Any + Send + 'static>(&self) -> Option<&A> {
        self.val.downcast_ref()
    }
}

/// A marker value to indicate a yield or resume with no value.
///
/// This exists to unify the implementations of the various operators, and should only ever be
/// created by internal code.
#[derive(Debug)]
pub(crate) struct EmptyYieldVal;

fn default_fatal_handler(inst: &Instance) -> ! {
    panic!("> instance {:p} had fatal error: {}", inst, inst.state);
}
