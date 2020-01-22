use anyhow::Error;
use lucet_module::bindings::Bindings;
use lucet_runtime_internals::module::DlModule;
use lucet_wasi_sdk::{CompileOpts, Link, LinkOpt, LinkOpts};
use lucetc::{Lucetc, LucetcOpts};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

pub fn test_module_c(dir: &str, cfile: &str) -> Result<Arc<DlModule>, Error> {
    let c_path = guest_file(dir, cfile);
    let bindings_path = guest_file(dir, "bindings.json");
    c_test(c_path, bindings_path)
}

pub fn guest_file(dir: &str, fname: &str) -> PathBuf {
    let root = env!("CARGO_MANIFEST_DIR");
    let mut p = PathBuf::from(root);
    p.push("guests");
    p.push(dir);
    p.push(fname);
    assert!(p.exists(), "test case source file {}/{} exists", dir, fname);
    p
}

pub fn c_test<P, Q>(c_file: P, bindings_file: Q) -> Result<Arc<DlModule>, Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let workdir = TempDir::new().expect("create working directory");

    let wasm_build = Link::new(&[c_file])
        .with_cflag("-nostartfiles")
        .with_link_opt(LinkOpt::NoDefaultEntryPoint)
        .with_link_opt(LinkOpt::AllowUndefinedAll)
        .with_link_opt(LinkOpt::ExportAll);

    let wasm_file = workdir.path().join("out.wasm");

    wasm_build.link(wasm_file.clone())?;

    let bindings = Bindings::from_file(bindings_file.as_ref())?;

    let native_build = Lucetc::new(wasm_file).with_bindings(bindings);

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}

pub fn test_module_wasm(dir: &str, wasmfile: &str) -> Result<Arc<DlModule>, Error> {
    let wasm_path = guest_file(dir, wasmfile);
    let bindings_path = guest_file(dir, "bindings.json");
    wasm_test(wasm_path, bindings_path)
}

pub fn wasm_test<P, Q>(wasm_file: P, bindings_file: Q) -> Result<Arc<DlModule>, Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let workdir = TempDir::new().expect("create working directory");

    let bindings = Bindings::from_file(&bindings_file)?;

    let native_build = Lucetc::new(wasm_file).with_bindings(bindings);

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}
