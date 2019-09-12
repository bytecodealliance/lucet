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
