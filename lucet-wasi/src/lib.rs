#![deny(bare_trait_objects)]

mod bindings;
pub mod c_api;
pub mod wasi;

// Lucet-specific wrappers of wasi-common:
pub use bindings::bindings;
pub use wasi::export_wasi_funcs;

// Wasi-common re-exports:
pub use wasi_common::{wasi::__wasi_exitcode_t, Error, WasiCtx, WasiCtxBuilder};

// Wasi executables export the following symbol for the entry point:
pub const START_SYMBOL: &'static str = "_start";
