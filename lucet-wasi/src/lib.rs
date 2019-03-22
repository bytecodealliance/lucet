pub mod c_api;
pub mod ctx;
pub mod fdentry;
pub mod host;
pub mod hostcalls;
pub mod memory;
pub mod wasm32;

pub use ctx::{WasiCtx, WasiCtxBuilder};

/// Call this if you're having trouble with `__wasi_*` symbols not being exported.
///
/// This is pretty hackish; we will hopefully be able to avoid this altogether once [this
/// issue](https://github.com/rust-lang/rust/issues/58037) is addressed.
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn lucet_wasi_internal_ensure_linked() {
    self::hostcalls::ensure_linked();
}

