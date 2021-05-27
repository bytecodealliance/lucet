//! # Lucet Runtime for Sandboxed WebAssembly Applications
//!
//! This crate runs programs that were compiled with the `lucetc` WebAssembly to native code
//! compiler. It provides an interface for modules to be loaded from shared object files (see
//! `DlModule`), and for hosts to provide specialized functionality to guests (see
//! `Instance::embed_ctx()`).
//!
//! The runtime is a critical part of the safety and security story for Lucet. While the semantics
//! of WebAssembly and the `lucetc` compiler provide many guarantees, the runtime must be correct in
//! order for the assumptions of those guarantees to hold. For example, the runtime uses guard pages
//! to ensure that any attempts by guest programs to access memory past the end of the guest heap are
//! safely caught.
//!
//! The runtime is also extensible, and some of the key types are defined as traits for
//! flexibility. See the `lucet-runtime-internals` crate for details.
//!
//! ## Running a Lucet Program
//!
//! There are a few essential types for using the runtime:
//!
//! - [`Instance`](struct.Instance.html): a Lucet program, together with its dedicated memory and
//! signal handlers. Users of this API never own an `Instance` directly, but can own the
//! [`InstanceHandle`](struct.InstanceHandle.html) smart pointer.
//!
//! - [`Region`](trait.Region.html): the memory from which instances are created. This crate
//! includes [`MmapRegion`](struct.MmapRegion.html), an implementation backed by `mmap`, and
//! optionally [`UffdRegion`](struct.UffdRegion.html), which is backed by the
//! [`userfaultfd`](http://man7.org/linux/man-pages/man2/userfaultfd.2.html) feature available on
//! newer Linux kernels ([see below](index.html#userfaultfd-backed-region)).
//!
//! - [`Limits`](struct.Limits.html): upper bounds for the resources a Lucet instance may
//! consume. These may be larger or smaller than the limits described in the WebAssembly module
//! itself; the smaller limit is always enforced.
//!
//! - [`Module`](trait.Module.html): the read-only parts of a Lucet program, including its code and
//! initial heap configuration. This crate includes [`DlModule`](struct.DlModule.html), an
//! implementation backed by dynamic loading of shared objects.
//!
//! - [`Val`](enum.Val.html): an enum describing values in WebAssembly, used to provide
//! arguments. These can be created using `From` implementations of primitive types, for example
//! `5u64.into()` in the example below.
//!
//! - [`RunResult`](enum.RunResult.html): the result of running or resuming an instance. These
//! contain either `UntypedRetVal`s for WebAssembly functions that have returned, or `YieldedVal`s
//! for WebAssembly programs that have yielded.
//!
//! - [`UntypedRetVal`](struct.UntypedRetVal.html): values returned from WebAssembly
//! functions. These must be interpreted at the correct type by the user via `From` implementations
//! or `retval.as_T()` methods, for example `u64::from(retval)` in the example below.
//!
//! - [`YieldedVal`](struct.YieldedVal.html): dynamically-values yielded by WebAssembly
//! programs. Not all yield points are given values, so this may be empty. To use the values, if
//! present, you must first downcast them with the provided methods.
//!
//! To run a Lucet program, you start by creating a region, capable of backing a number of
//! instances. You then load a module and then create a new instance using the region and the
//! module. You can then run any of the functions that the Lucet program exports, retrieve return
//! values from those functions, and access the linear memory of the guest.
//!
//! ```no_run
//! use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
//!
//! let module = DlModule::load("/my/lucet/module.so").unwrap();
//! let region = MmapRegion::create(1, &Limits::default()).unwrap();
//! let mut inst = region.new_instance(module).unwrap();
//!
//! let retval = inst.run("factorial", &[5u64.into()]).unwrap().unwrap_returned();
//! assert_eq!(u64::from(retval), 120u64);
//! ```
//!
//! ## Embedding With Hostcalls
//!
//! A "hostcall" is a function called by WebAssembly that is not defined in WebAssembly. Since
//! WebAssembly is such a minimal language, hostcalls are required for Lucet programs to do anything
//! interesting with the outside world. For example, in Fastly's [Terrarium
//! demo](https://wasm.fastly-labs.com/), hostcalls are provided for manipulating HTTP requests,
//! accessing a key/value store, etc.
//!
//! Some simple hostcalls can be implemented by using the
//! [`#[lucet_hostcall]`](attr.lucet_hostcall.html) attribute on a function that takes `&Vmctx` as
//! its first argument. Hostcalls that require access to some embedder-specific state, such as
//! Terrarium's key-value store, can access a custom embedder context through `vmctx`. For example,
//! to make a `u32` available to hostcalls:
//!
//! ```no_run
//! use lucet_runtime::{DlModule, Limits, MmapRegion, Region, lucet_hostcall};
//! use lucet_runtime::vmctx::{Vmctx, lucet_vmctx};
//!
//! struct MyContext { x: u32 }
//!
//! #[lucet_hostcall]
//! #[no_mangle]
//! pub fn foo(vmctx: &Vmctx) {
//!     let mut hostcall_context = vmctx.get_embed_ctx_mut::<MyContext>();
//!     hostcall_context.x = 42;
//! }
//!
//! let module = DlModule::load("/my/lucet/module.so").unwrap();
//! let region = MmapRegion::create(1, &Limits::default()).unwrap();
//! let mut inst = region
//!     .new_instance_builder(module)
//!     .with_embed_ctx(MyContext { x: 0 })
//!     .build()
//!     .unwrap();
//!
//! inst.run("call_foo", &[]).unwrap();
//!
//! let context_after = inst.get_embed_ctx::<MyContext>().unwrap().unwrap();
//! assert_eq!(context_after.x, 42);
//! ```
//!
//! The embedder context is backed by a structure that can hold a single value of any type. Rust
//! embedders should add their own custom state type (like `MyContext` above) for any context they
//! require, rather than using a common type (such as the `u32`) from the standard library. This
//! avoids collisions between libraries, and allows for easy composition of embeddings.
//!
//! For C-based embedders, the type `*mut libc::c_void` is privileged as the only type that the C
//! API provides. The following example shows how a Rust embedder can initialize a C-compatible
//! context:
//!
//! ```no_run
//! use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
//!
//! let module = DlModule::load("/my/lucet/module.so").unwrap();
//! let region = MmapRegion::create(1, &Limits::default()).unwrap();
//! #[repr(C)]
//! struct MyForeignContext { x: u32 };
//! let mut foreign_ctx = Box::into_raw(Box::new(MyForeignContext{ x: 0 }));
//! let mut inst = region
//!     .new_instance_builder(module)
//!     .with_embed_ctx(foreign_ctx as *mut libc::c_void)
//!     .build()
//!     .unwrap();
//!
//! inst.run("main", &[]).unwrap();
//!
//! // clean up embedder context
//! drop(inst);
//! // foreign_ctx must outlive inst, but then must be turned back into a box
//! // in order to drop.
//! unsafe { Box::from_raw(foreign_ctx) };
//! ```
//!
//! ## Yielding and Resuming
//!
//! Lucet hostcalls can use the `vmctx` argument to yield, suspending themselves and optionally
//! returning a value back to the host context. A yielded instance can then be resumed by the host,
//! and execution will continue from the point of the yield.
//!
//! Four yield methods are available for hostcall implementors:
//!
//! |                                                                                     | Yields value? | Expects value? |
//! |-------------------------------------------------------------------------------------|---------------|----------------|
//! | [`yield_`](vmctx/struct.Vmctx.html#method.yield_)                                   | ❌            | ❌             |
//! | [`yield_val`](vmctx/struct.Vmctx.html#method.yield_val)                             | ✅             | ❌             |
//! | [`yield_expecting_val`](vmctx/struct.Vmctx.html#method.yield_expecting_val)         | ❌            | ✅              |
//! | [`yield_val_expecting_val`](vmctx/struct.Vmctx.html#method.yield_val_expecting_val) | ✅             | ✅              |
//!
//! The host is free to ignore values yielded by guests, but a yielded instance may only be resumed
//! with a value of the correct type using
//! [`Instance::resume_with_val()`](struct.Instance.html#method.resume_with_val), if one is
//! expected.
//!
//! ### Factorial example
//!
//! In this example, we use yielding and resuming to offload multiplication to the host context, and
//! to incrementally return results to the host. While certainly overkill for computing a factorial
//! function, this structure mirrors that of many asynchronous workflows.
//!
//! Since the focus of this example is on the behavior of hostcalls that yield, our Lucet guest
//! program just invokes a hostcall:
//!
//! ```no_run
//! // factorials_guest.rs
//! extern "C" {
//!     fn hostcall_factorials(n: u64) -> u64;
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn run() -> u64 {
//!     unsafe {
//!         hostcall_factorials(5)
//!     }
//! }
//! ```
//!
//! In our hostcall, there are two changes from a standard recursive implementation of factorial.
//!
//! - Instead of performing the `n * fact(n - 1)` multiplication ourselves, we yield the operands
//! and expect the product when resumed.
//!
//! - Whenever we have computed a factorial, including both intermediate values and the final
//! answer, we yield it.
//!
//! The final answer is returned normally as the result of the guest function.
//!
//! To implement this, we introduce a new `enum` type to represent what we want the host to do next,
//! and yield it when appropriate.
//!
//! ```no_run
//! use lucet_runtime::lucet_hostcall;
//! use lucet_runtime::vmctx::Vmctx;
//!
//! pub enum FactorialsK {
//!     Mult(u64, u64),
//!     Result(u64),
//! }
//!
//! #[lucet_hostcall]
//! #[no_mangle]
//! pub fn hostcall_factorials(vmctx: &Vmctx, n: u64) -> u64 {
//!     fn fact(vmctx: &Vmctx, n: u64) -> u64 {
//!         let result = if n <= 1 {
//!             1
//!         } else {
//!             let n_rec = fact(vmctx, n - 1);
//!             // yield a request for the host to perform multiplication
//!             vmctx.yield_val_expecting_val(FactorialsK::Mult(n, n_rec))
//!             // once resumed, that yield evaluates to the multiplication result
//!         };
//!         // yield a result
//!         vmctx.yield_val(FactorialsK::Result(result));
//!         result
//!     }
//!     fact(vmctx, n)
//! }
//! ```
//!
//! The host side of the code, then, is an interpreter that repeatedly checks the yielded value and
//! performs the appropriate operation. The hostcall returns normally with the final answer when it
//! is finished, so we exit the loop when the run/resume result is `Ok`.
//!
//! ```no_run
//! # pub enum FactorialsK {
//! #     Mult(u64, u64),
//! #     Result(u64),
//! # }
//! use lucet_runtime::{DlModule, Error, Limits, MmapRegion, Region};
//!
//! let module = DlModule::load("factorials_guest.so").unwrap();
//! let region = MmapRegion::create(1, &Limits::default()).unwrap();
//! let mut inst = region.new_instance(module).unwrap();
//!
//! let mut factorials = vec![];
//!
//! let mut res = inst.run("run", &[]).unwrap();
//!
//! while let Ok(val) = res.yielded_ref() {
//!     if let Some(k) = val.downcast_ref::<FactorialsK>() {
//!         match k {
//!             FactorialsK::Mult(n, n_rec) => {
//!                 // guest wants us to multiply for it
//!                 res = inst.resume_with_val(n * n_rec).unwrap();
//!             }
//!             FactorialsK::Result(n) => {
//!                 // guest is returning an answer
//!                 factorials.push(*n);
//!                 res = inst.resume().unwrap();
//!             }
//!         }
//!     } else {
//!         panic!("didn't yield with expected type");
//!     }
//! }
//!
//! // intermediate values are correct
//! assert_eq!(factorials.as_slice(), &[1, 2, 6, 24, 120]);
//! // final value is correct
//! assert_eq!(u64::from(res.unwrap_returned()), 120u64);
//! ```
//!
//! ## Custom Signal Handlers
//!
//! Since Lucet programs are run as native machine code, signals such as `SIGSEGV` and `SIGFPE` can
//! arise during execution. Rather than letting these signals bring down the entire process, the
//! Lucet runtime installs alternate signal handlers that limit the effects to just the instance
//! that raised the signal.
//!
//! By default, the signal handler sets the instance state to `State::Fault` and returns early from
//! the call to `Instance::run()`. You can, however, implement custom error recovery and logging
//! behavior by defining new signal handlers on a per-instance basis. For example, the following
//! signal handler increments a counter of signals it has seen before setting the fault state:
//!
//! ```no_run
//! use lucet_runtime::{
//!     DlModule, Error, Instance, Limits, MmapRegion, Region, SignalBehavior, TrapCode,
//! };
//! use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
//!
//! static SIGNAL_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
//!
//! fn signal_handler_count(
//!     _inst: &Instance,
//!     _trapcode: &Option<TrapCode>,
//!     _signum: libc::c_int,
//!     _siginfo_ptr: *const libc::siginfo_t,
//!     _ucontext_ptr: *const libc::c_void,
//! ) -> SignalBehavior {
//!     SIGNAL_COUNT.fetch_add(1, Ordering::SeqCst);
//!     SignalBehavior::Default
//! }
//!
//! let module = DlModule::load("/my/lucet/module.so").unwrap();
//! let region = MmapRegion::create(1, &Limits::default()).unwrap();
//! let mut inst = region.new_instance(module).unwrap();
//!
//! // install the handler
//! inst.set_signal_handler(signal_handler_count);
//!
//! match inst.run("raise_a_signal", &[]) {
//!     Err(Error::RuntimeFault(_)) => {
//!         println!("I've now handled {} signals!", SIGNAL_COUNT.load(Ordering::SeqCst));
//!     }
//!     res => panic!("unexpected result: {:?}", res),
//! }
//! ```
//!
//! When implementing custom signal handlers for the Lucet runtime, the usual caveats about signal
//! safety apply: see
//! [`signal-safety(7)`](http://man7.org/linux/man-pages/man7/signal-safety.7.html).
//!
//! ## Interaction With Host Signal Handlers
//!
//! Great care must be taken if a host application installs or otherwise modifies signal handlers
//! anywhere in the process. Lucet installs handlers for `SIGBUS`, `SIGFPE`, `SIGILL`, and `SIGSEGV`
//! when the first Lucet instance begins running, and restores the preëxisting handlers when the
//! last Lucet instance terminates. During this time, other threads in the host process *must not*
//! modify those signal handlers, since signal handlers can only be installed on a process-wide
//! basis.
//!
//! Despite this limitation, Lucet is designed to compose with other signal handlers in the host
//! program. If one of the above signals is caught by the Lucet signal handler, but that thread is
//! not currently running a Lucet instance, the saved host signal handler is called. This means
//! that, for example, a `SIGSEGV` on a non-Lucet thread of a host program will still likely abort
//! the entire process.
//!
//! For advanced uses that require manual control over the installation and removal of the Lucet
//! signal handler, you can use [`install_lucet_signal_handler()`][install-handler] and
//! [`remove_lucet_signal_handler()`][remove-handler], and disable the automatic installation and
//! removal on a per-instance basis by setting
//! [instance.`ensure_signal_handler_installed(false)`][instance-ensure-handler]
//!
//! [install-handler]: fn.install_lucet_signal_handler.html
//! [remove-handler]: fn.remove_lucet_signal_handler.html
//! [instance-ensure-handler]: struct.Instance.html#method.ensure_signal_handler_installed
//!
//! ## Signal Handler Stacks
//!
//! Lucet instances must run on threads that have an [alternate signal stack][sigaltstack]
//! installed, otherwise a WebAssembly program that overflows its stack will cause a double fault,
//! crashing the entire process. By default, Lucet instances install an alternate signal stack on
//! their thread before running, and reinstall the preëxisting signal stack on exit. Space for this
//! stack is reserved within the Lucet `Region` as defined by the `signal_stack_size` field of
//! `Limits`. If `signal_stack_size` isn't large enough (see below), the automatic installation will
//! fail with an `Error::InvalidArgument` value.
//!
//! For advanced uses, the automatic signal stack installation can be disabled on a per-instance
//! basis by setting [`instance::ensure_sigstack_installed(false)`][instance-ensure-sigstack]. In
//! debug mode, the runtime will still check that an alternate signal stack is present and of
//! sufficient size, but in release mode no checks or extra system calls will be performed.
//!
//! [sigaltstack]: http://man7.org/linux/man-pages/man2/sigaltstack.2.html
//! [instance-ensure-sigstack]: struct.Instance.html#method.ensure_sigstack_installed
//!
//! ### Signal Stack Size
//!
//! The alternate signal stack must be of a sufficient size to avoid stack overflows when running
//! the signal handler, and to satisfy [`sigaltstack`][sigaltstack]'s requirements. This library
//! provides the constant [`DEFAULT_SIGNAL_STACK_SIZE`][default-sigstack-size] as a recommendation
//! for signal stack size, and uses this value as the default when creating `Limits`.
//!
//! With Rust optimizations enabled, as with `cargo build --release`, the `sigaltstack` requirement
//! of `MINSIGSTKSZ` (defined in `<signal.h>`) provides sufficient space for the signal handler.
//!
//! With no optimizations, as with `cargo build`, the signal stack size must be at least 12KiB
//! according to our experiments; since this is dependent on compiler-defined memory layout choices,
//! this number could change between Lucet releases or even Rust compiler versions.
//!
//! [default-sigstack-size]: constant.DEFAULT_SIGNAL_STACK_SIZE.html
//!
//! ## `userfaultfd`-Backed Region
//!
//! [`UffdRegion`](struct.UffdRegion.html) is a [`Region`](trait.Region.html) backed by the
//! [`userfaultfd`](http://man7.org/linux/man-pages/man2/userfaultfd.2.html) feature available in
//! newer Linux kernels. It allows Lucet instances to lazily copy in the initial heap contents of an
//! `Instance`, reducing startup time. Instance stack pages can also be lazily initialized, reducing
//! the memory footprint of instances that only use a small portion of their available stack space.
//!
//! `UffdRegion` is enabled by default on Linux platforms, but can be disabled by disabling default
//! features for this crate and `lucet-runtime-internals`:
//!
//! ```toml
//! [dependencies]
//! lucet-runtime = { version = "0.6.1", default-features = false }
//! lucet-runtime-internals = { version = "0.6.1", default-features = false }
//! ```

#![deny(bare_trait_objects)]

// This makes `lucet_runtime` in the expansion of `#[lucet_hostcall]` resolve to something
// meaningful when used in this crate.
extern crate self as lucet_runtime;

pub mod c_api;

pub use lucet_module::{PublicKey, TrapCode};
pub use lucet_runtime_internals::alloc::{AllocStrategy, Limits, DEFAULT_SIGNAL_STACK_SIZE};
pub use lucet_runtime_internals::error::Error;
pub use lucet_runtime_internals::future::RunAsync;
pub use lucet_runtime_internals::instance::signals::{
    install_lucet_signal_handler, remove_lucet_signal_handler,
};
pub use lucet_runtime_internals::instance::{
    FaultDetails, Instance, InstanceHandle, KillError, KillSuccess, KillSwitch, RunResult,
    SignalBehavior, TerminationDetails, YieldedVal,
};
#[allow(deprecated)]
pub use lucet_runtime_internals::lucet_hostcalls;
pub use lucet_runtime_internals::module::{DlModule, Module};
pub use lucet_runtime_internals::region::mmap::MmapRegion;
#[cfg(all(target_os = "linux", feature = "uffd"))]
pub use lucet_runtime_internals::region::uffd::{
    Disposition as UffdDisposition, HeapPageSize as UffdHeapPageSize, UffdConfig, UffdRegion,
};
pub use lucet_runtime_internals::region::{InstanceBuilder, Region, RegionCreate};
pub use lucet_runtime_internals::val::{UntypedRetVal, Val};
pub use lucet_runtime_internals::{lucet_hostcall, lucet_hostcall_terminate, WASM_PAGE_SIZE};

pub mod vmctx {
    //! Functions for manipulating instances from hostcalls.
    //!
    //! The Lucet compiler inserts an extra `*const lucet_vmctx` argument to all functions defined
    //! and called by WebAssembly code. Through this pointer, code running in the guest context can
    //! access and manipulate the instance and its structures. These functions are intended for use
    //! in hostcall implementations, and must only be used from within a running guest.
    //!
    //! # Panics
    //!
    //! All of the `Vmctx` methods will panic if the `Vmctx` was not created from a valid pointer
    //! associated with a running instance. This should never occur if run in guest code on the
    //! pointer argument inserted by the compiler.
    pub use lucet_runtime_internals::vmctx::{lucet_vmctx, Vmctx};

    // must be exported for `lucet_hostcall`, but we don't want to advertise it
    #[doc(hidden)]
    pub use lucet_runtime_internals::vmctx::VmctxInternal;
}

/// Call this if you're having trouble with `lucet_*` symbols not being exported.
///
/// This is pretty hackish; we will hopefully be able to avoid this altogether once [this
/// issue](https://github.com/rust-lang/rust/issues/58037) is addressed.
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn lucet_internal_ensure_linked() {
    self::c_api::ensure_linked();
}
