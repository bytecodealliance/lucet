use tempdir::TempDir;
use std::path::PathBuf;
use std::sync::Arc;
use failure::Error;
use lucet_runtime_internals::module::DlModule;
use lucet_wasi_sdk::Link;
use lucetc::{Lucetc, Bindings};

pub fn test_module(dir: &str, cfile: &str) -> Result<Arc<DlModule>, Error> {
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

pub fn c_test(c_file: PathBuf, bindings_file: PathBuf) -> Result<Arc<DlModule>, Error> {

    let workdir = TempDir::new("c_test").expect("create working directory");

    let wasm_build = Link::new(vec![c_file])
        .cflag("-nostartfiles")
        .ldflag("--no-entry")
        .ldflag("--allow-undefined")
        .ldflag("--export-all");

    let wasm_file = workdir.path().join("out.wasm");

    wasm_build.link(wasm_file.clone())?;

    let bindings = Bindings::from_file(&bindings_file)?;

    let native_build = Lucetc::new(wasm_file)?
        .bindings(bindings)?;

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}


