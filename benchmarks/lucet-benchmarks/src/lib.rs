// This crate uses several Criterion APIs which are deprecated. However, the
// Lucet project is in maintence mode and we no longer really care about
// maintaining and updating these interfaces, so we will ignore these
// warnings:
#![allow(deprecated)]

mod compile;
mod context;
mod modules;
mod par;
mod seq;

pub use compile::compile_benches;
pub use context::context_benches;
pub use par::par_benches;
pub use seq::seq_benches;

#[no_mangle]
extern "C" fn lucet_benchmarks_ensure_linked() {
    lucet_runtime::lucet_internal_ensure_linked();
    lucet_wasi::export_wasi_funcs();
}
