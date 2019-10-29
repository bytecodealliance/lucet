#![deny(bare_trait_objects)]

mod bindings;
pub mod c_api;
pub mod wasi;

pub use bindings::bindings;
pub use wasi::{export_wasi_funcs, WasiCtx, WasiCtxBuilder};

// Exporting this type alias as a stop-gap: wasi-common should really provide a pub Rust enum type
// for the return values from _start, but it does not yet. We're going to pursue that path via witx
// and friends rather than export the type definition and constants from wasi-common right now.
//
// In the meantime, we're providing this alias because it is more descriptive to use (compared to
// `u32` in our code that uses the exitcode.
#[allow(non_camel_case_types)]
pub type __wasi_exitcode_t = u32;
