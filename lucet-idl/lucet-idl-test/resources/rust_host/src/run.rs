use crate::{harness, idl};
use failure::Error;
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::{self, WasiCtxBuilder};
use std::path::PathBuf;
use std::sync::Arc;

pub fn run(module_path: PathBuf) -> Result<(), Error> {
    lucet_wasi::hostcalls::ensure_linked();
    idl::ensure_linked();

    let module = DlModule::load(&module_path)?;

    let region = MmapRegion::create(
        1,
        &Limits {
            heap_memory_size: 4 * 1024 * 1024 * 1024,
            heap_address_space_size: 8 * 1024 * 1024 * 1024,
            globals_size: 4 * 1024 * 1024,
            stack_size: 4 * 1024 * 1024,
        },
    )?;

    let ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .build()
        .expect("create empty wasi ctx");

    let harness_ctx = harness::ctx();

    let mut inst = region
        .new_instance_builder(module as Arc<dyn Module>)
        .with_embed_ctx(ctx)
        .with_embed_ctx(harness_ctx)
        .build()
        .expect("construct instance");

    inst.run("_start", &[])?;

    Ok(())
}
