#![deny(bare_trait_objects)]

pub mod c_api;
pub mod ctx;
pub mod fdentry;
pub mod host;
pub mod hostcalls;
pub mod memory;
pub mod wasm32;

pub use ctx::{WasiCtx, WasiCtxBuilder};
