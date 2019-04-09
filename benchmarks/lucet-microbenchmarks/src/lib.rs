#[macro_use]
extern crate criterion;

use criterion::Criterion;
use lucet_runtime::{DlModule, InstanceHandle, Limits, MmapRegion, Module, Region};
use lucet_wasi_sdk::{CompileOpts, Lucetc};
use lucetc::{Bindings, LucetcOpts};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn wasi_bindings() -> Bindings {
    Bindings::from_file("../../lucet-wasi/bindings.json").unwrap()
}

fn compile_hello<P: AsRef<Path>>(so_file: P) {
    let wasm_build = Lucetc::new(&["../../lucet-wasi/examples/hello.c"])
        .print_output(true)
        .with_cflag("-Wall")
        .with_cflag("-Werror")
        .with_bindings(wasi_bindings());

    wasm_build.build(&so_file).unwrap();
}

fn load_mkregion_and_instantiate_body(so_file: &Path) -> InstanceHandle {
    let module = DlModule::load(so_file).unwrap();
    let region = MmapRegion::create(1, &Limits::default()).unwrap();
    region.new_instance(module).unwrap()
}

fn load_mkregion_and_instantiate(c: &mut Criterion) {
    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file);

    c.bench_function("load_mkregion_and_instantiate hello", move |b| {
        b.iter(|| load_mkregion_and_instantiate_body(&so_file))
    });

    workdir.close().unwrap();
}

fn instantiate_body(module: Arc<dyn Module>, region: Arc<MmapRegion>) -> InstanceHandle {
    region.new_instance(module).unwrap()
}

fn instantiate(c: &mut Criterion) {
    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");
    compile_hello(&so_file);

    let module = DlModule::load(&so_file).unwrap();
    let region = MmapRegion::create(1, &Limits::default()).unwrap();

    c.bench_function("instantiate hello", move |b| {
        b.iter(|| instantiate_body(module.clone(), region.clone()))
    });

    workdir.close().unwrap();
}

criterion_group!(benches, load_mkregion_and_instantiate, instantiate);

#[no_mangle]
extern "C" fn lucet_microbenchmarks_ensure_linked() {
    lucet_runtime::lucet_internal_ensure_linked();
    lucet_wasi::hostcalls::ensure_linked();
}
