use lucet_runtime::{DlModule, Limits, MmapRegion, Region};
use lucet_wasi::WasiCtxBuilder;

#[tokio::main]
async fn main() {
    // ensure the WASI symbols are exported from the final executable
    lucet_wasi::export_wasi_funcs();
    // load the compiled Lucet module
    let dl_module = DlModule::load("example.so").unwrap();
    // create a new memory region with default limits on heap and stack size
    let region = MmapRegion::create(1, &Limits::default()).unwrap();
    // instantiate the module in the memory region
    let mut instance = region.new_instance(dl_module).unwrap();
    // prepare the WASI context, inheriting stdio handles from the host executable.
    // Since we are using lucet-wasi, we need to run the instance as async!
    let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();
    instance.insert_embed_ctx(wasi_ctx);
    // run the WASI entrypoint.
    instance.run_async("_start", &[], None).await.unwrap();
}
