use failure::Error;
use lucet_runtime::{DlModule, Module};
use lucet_runtime::{Limits, MmapRegion, Region};
use lucet_wasi::WasiCtx;
use lucet_wasi_sdk::Link;
use lucetc::{Bindings, Lucetc};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

pub const LUCET_WASI_ROOT: &'static str = env!("CARGO_MANIFEST_DIR");

pub fn test_module_wasi<P: AsRef<Path>>(cfile: P) -> Result<Arc<dyn Module>, Error> {
    let c_path = guest_file(&cfile);
    wasi_test(c_path)
}

pub fn guest_file<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = if path.as_ref().is_absolute() {
        path.as_ref().to_owned()
    } else {
        Path::new(LUCET_WASI_ROOT)
            .join("tests")
            .join("guests")
            .join(path)
    };
    assert!(p.exists(), "test case source file {} exists", p.display());
    p
}

pub fn wasi_test<P: AsRef<Path>>(c_file: P) -> Result<Arc<dyn Module>, Error> {
    let workdir = TempDir::new().expect("create working directory");

    let wasm_build = Link::new(&[c_file]);

    let wasm_file = workdir.path().join("out.wasm");

    wasm_build.link(wasm_file.clone())?;

    let bindings = Bindings::from_file(Path::new(LUCET_WASI_ROOT).join("bindings.json"))?;

    let native_build = Lucetc::new(wasm_file)?.bindings(bindings)?;

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule as Arc<dyn Module>)
}

pub fn run_with_stdout<P: AsRef<Path>>(path: P, mut ctx: WasiCtx) -> Result<String, Error> {
    let region = MmapRegion::create(1, &Limits::default())?;
    let module = test_module_wasi(path)?;

    let (pipe_out, pipe_in) = nix::unistd::pipe()?;
    ctx.insert_existing_fd(1, pipe_in);

    let mut inst = region
        .new_instance_builder(module)
        .with_embed_ctx(ctx)
        .build()?;
    inst.run(b"_start", &[])?;

    nix::unistd::close(pipe_in)?;

    let mut stdout_file = unsafe { File::from_raw_fd(pipe_out) };
    let mut stdout = String::new();
    stdout_file.read_to_string(&mut stdout)?;
    nix::unistd::close(stdout_file.into_raw_fd())?;

    Ok(stdout)
}
