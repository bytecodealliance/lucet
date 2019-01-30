//! # Runtime for Sandboxed WebAssembly Applications
//!
//! This crate runs programs that were compiled with the `lucetc` WebAssembly to native code
//! compiler. It provides an interface for modules to be loaded from shared object files (see
//! `DlModule`), and for hosts to provide specialized functionality to guests (see
//! `Region::new_instance_with_ctx`).
//!
//! The runtime is a critical part of the safety and security story for Lucet. While the semantics
//! of WebAssembly and the `lucetc` compiler provide many guarantees, the runtime must be correct in
//! order for the assumptions of those guarantees to hold. For example, the runtime uses guard pages
//! to ensure that any attempts by guest programs to access memory past the end of the guest heap are
//! safely caught.
//!
//! ## Running a Lucet Program
//!
//! There are a few essential types for using the runtime:
//!
//! - `Instance`: a Lucet program, together with its dedicated memory and signal handlers.
//!
//! - `Region`: the memory from which instances are created. This crate includes an implementation
//! backed by `mmap`.
//!
//! - `Limits`: upper bounds for the resources a Lucet instance may consume. These may be larger or
//! smaller than the limits described in the WebAssembly module itself; the smaller limit is always
//! enforced.
//!
//! - `Module`: the read-only parts of a Lucet program, including its code and initial heap
//! configuration. This crate includes an implementation backed by dynamic loading of shared
//! objects.
//!
//! - `Val`: an enum describing values in WebAssembly, used to provide arguments and read return
//! vavlues.
//!
//! To run a Lucet program, you start by creating a region, capable of backing a number of
//! instances. You then load a module and then create a new instance using the region and the
//! module. You can then run any of the functions that the Lucet program exports, retrieve return
//! values from those functions, and even access the linear memory of the guest.
//!
//! ```no_run
//! use lucet_runtime::instance::State;
//! use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
//! let module = DlModule::load("/my/lucet/module.so").unwrap();
//! let region = MmapRegion::create(1, &Limits::default()).unwrap();
//! let mut inst = region.new_instance(Box::new(module)).unwrap();
//!
//! inst.run(b"factorial", &[5u64.into()]).unwrap();
//!
//! match &inst.state {
//!     State::Ready { retval } => {
//!         assert_eq!(u64::from(retval), 120u64);
//!     }
//!     _ => panic!("unexpected final state: {}", inst.state),
//! }
//! ```

pub use lucet_runtime_internals::alloc::Limits;
pub use lucet_runtime_internals::instance::{self, Instance, WASM_PAGE_SIZE};
pub use lucet_runtime_internals::module::{self, DlModule, Module};
pub use lucet_runtime_internals::region::mmap::MmapRegion;
pub use lucet_runtime_internals::region::Region;
pub use lucet_runtime_internals::val::Val;
pub use lucet_runtime_internals::vmctx::Vmctx;
