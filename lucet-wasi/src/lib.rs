#![deny(bare_trait_objects)]

#[cfg(feature = "runtime")]
pub mod c_api;
#[cfg(feature = "runtime")]
pub mod runtime;

#[cfg(feature = "runtime")]
pub use runtime::*;
// Wasi-common re-exports:
pub use wasi_common::{WasiCtx, WasiCtxBuilder};

// Wasi executables export the following symbol for the entry point:
pub const START_SYMBOL: &str = "_start";

pub fn bindings() -> lucet_module::bindings::Bindings {
    lucet_wiggle_generate::bindings(&wasi_common::wasi::metadata::document())
}

pub fn document() -> wiggle::witx::Document {
    wasi_common::wasi::metadata::document()
}
