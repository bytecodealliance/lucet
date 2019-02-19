//! `lucet-runtime` is a library for loading, running, and monitoring ahead-of-time compiled
//! WebAssembly modules in lightweight sandboxes. It is intended to work with modules compiled by
//! [`lucetc`](https://github.com/fastly/lucet/tree/master/lucetc).

#![deny(bare_trait_objects)]

#[macro_use]
pub mod error;

#[macro_use]
#[cfg(test)]
pub mod test_helpers;

pub mod alloc;
pub mod context;
pub mod instance;
pub mod module;
pub mod probestack;
pub mod region;
pub mod trapcode;
pub mod val;
pub mod vmctx;

/// The size of a page in WebAssembly heaps.
pub const WASM_PAGE_SIZE: u32 = 64 * 1024;
