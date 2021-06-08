use anyhow::{anyhow, bail, Error};
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::{self, types::Exitcode, WasiCtx, WasiCtxBuilder};
use lucet_wasi_sdk::{CompileOpts, Link};
use lucetc::{Lucetc, LucetcOpts, Validator, WasiMode};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

pub fn init_tracing() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init()
    })
}

pub const LUCET_WASI_ROOT: &str = env!("CARGO_MANIFEST_DIR");

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

pub fn wasi_test<P: AsRef<Path>>(file: P) -> Result<Arc<dyn Module>, Error> {
    let workdir = TempDir::new().expect("create working directory");

    let wasm_path = match file.as_ref().extension().and_then(|x| x.to_str()) {
        Some("c") => {
            // some tests are .c, and must be compiled/linked to .wasm we can run
            let wasm_build = Link::new(&[file])
                .with_cflag("-Wall")
                .with_cflag("-Werror")
                .with_print_output(true);

            let wasm_file = workdir.path().join("out.wasm");
            wasm_build.link(wasm_file.clone())?;

            wasm_file
        }
        Some("wasm") | Some("wat") => {
            // others are just wasm we can run directly
            file.as_ref().to_owned()
        }
        Some(ext) => {
            panic!("unknown test file extension: .{}", ext);
        }
        None => {
            panic!("unknown test file, has no extension");
        }
    };

    wasi_load(&workdir, wasm_path)
}

pub fn wasi_load<P: AsRef<Path>>(
    workdir: &TempDir,
    wasm_file: P,
) -> Result<Arc<dyn Module>, Error> {
    let native_build = Lucetc::new(wasm_file)
        .with_bindings(lucet_wasi::bindings())
        .with_validator(
            Validator::builder()
                .witx(lucet_wasi::witx_document())
                .wasi_mode(Some(WasiMode::Command))
                .build(),
        );

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule as Arc<dyn Module>)
}

pub fn run<P: AsRef<Path>>(path: P, ctx: WasiCtx) -> Result<Exitcode, Error> {
    let region = MmapRegion::create(1, &Limits::default())?;
    let module = test_module_wasi(path)?;

    let mut inst = region
        .new_instance_builder(module)
        .with_embed_ctx(ctx)
        .build()?;

    let r = tokio::runtime::Runtime::new()
        .expect("create runtime")
        .block_on(async move { inst.run_async("_start", &[], None).await });

    match r {
        // normal termination implies 0 exit code
        Ok(_) => Ok(0),
        Err(lucet_runtime::Error::RuntimeTerminated(details)) => details
            .as_exitcode()
            .ok_or_else(|| anyhow!("expected exitcode, got: {:?}", details,)),
        Err(e) => bail!("runtime error: {}", e),
    }
}

pub fn run_with_stdout<P: AsRef<Path>>(
    path: P,
    mut ctx: WasiCtxBuilder,
) -> Result<(Exitcode, String), Error> {
    let stdout = wasi_common::pipe::WritePipe::new_in_memory();
    ctx = ctx.stdout(Box::new(stdout.clone()));

    let ctx = ctx.build();

    let run_result = run(path, ctx);

    let stdout = String::from_utf8(
        stdout
            .try_into_inner()
            .map_err(|_| anyhow!("no other pipe references can exist"))?
            .into_inner(),
    )?;

    if !stdout.is_empty() {
        println!("guest stdout:\n{}", stdout);
    }

    // Delay erroring on the run result until stdout has been printed
    let exitcode = run_result?;
    Ok((exitcode, stdout))
}

pub fn run_with_null_stdin<P: AsRef<Path>>(
    path: P,
    mut ctx: WasiCtxBuilder,
) -> Result<Exitcode, Error> {
    let stdin = wasi_common::pipe::ReadPipe::new(std::io::empty());
    ctx = ctx.stdin(Box::new(stdin));

    let ctx = ctx.build();

    let exitcode = run(path, ctx)?;

    Ok(exitcode)
}

/// Call this if you're having trouble with `__wasi_*` symbols not being exported.
///
/// This is pretty hackish; we will hopefully be able to avoid this altogether once [this
/// issue](https://github.com/rust-lang/rust/issues/58037) is addressed.
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn lucet_wasi_tests_internal_ensure_linked() {
    lucet_runtime::lucet_internal_ensure_linked();
    lucet_wasi::export_wasi_funcs();
}
