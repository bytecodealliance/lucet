//! `lucet-runtime` is a library for loading, running, and monitoring
//! ahead-of-time compiled WebAssembly modules in lightweight
//! sandboxes. It is intended to work with modules compiled by
//! [`lucetc`](https://github.com/fastly/isolation/tree/master/lib/lucetc).

#![deny(bare_trait_objects)]

#[macro_use]
#[cfg(test)]
pub mod test_helpers;

pub mod alloc;
pub mod context;
pub mod instance;
pub mod module;
pub mod probestack;
pub mod region;
pub mod val;
pub mod vmctx;

pub use crate::alloc::Limits;
pub use crate::instance::{Instance, WASM_PAGE_SIZE};
pub use crate::module::{DlModule, Module};
pub use crate::region::Region;
pub use crate::val::Val;
pub use crate::vmctx::Vmctx;
