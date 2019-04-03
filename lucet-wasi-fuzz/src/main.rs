use failure::{bail, format_err, Error};
use libc::c_ulong;
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::host::__wasi_exitcode_t;
use lucet_wasi::{WasiCtx, WasiCtxBuilder};
use lucet_wasi_sdk::Link;
use lucetc::{Bindings, Lucetc};
use rand::prelude::random;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;
use tempfile::TempDir;
use wait_timeout::ChildExt;

const LUCET_WASI_FUZZ_ROOT: &'static str = env!("CARGO_MANIFEST_DIR");

type Seed = c_ulong;

#[derive(StructOpt, Debug)]
#[structopt(name = "lucet-wasi-fuzz")]
struct Config {
    #[structopt(short = "n", long = "num-tests", default_value = "100")]
    num_tests: usize,
    #[structopt(long = "seed")]
    seed: Option<Seed>,
}

fn main() {
    lucet_runtime::lucet_internal_ensure_linked();

    let config = Config::from_args();

    if let Some(seed) = config.seed {
        run_one_seed(seed);
    } else {
        run_many(config.num_tests);
    }
}

fn run_one_seed(seed: Seed) {
    match run_one(Some(seed)) {
        Ok(TestResult::Passed) => println!("test passed"),
        Ok(TestResult::Ignored) => println!("native build/execution failed"),
        Ok(TestResult::Failed {
            expected, actual, ..
        }) => {
            println!("test failed:\n");
            println!("native: {}", String::from_utf8_lossy(&expected));
            println!("lucet-wasi: {}", String::from_utf8_lossy(&actual));
            std::process::exit(1);
        }
        Ok(TestResult::Errored { error }) | Err(error) => println!("test errored: {}", error),
    }
}

fn run_many(num_tests: usize) {
    let mut progress = progress::Bar::new();
    progress.set_job_title(&format!("Running {} tests", num_tests));

    let progress = Arc::new(Mutex::new(progress));
    let num_finished = Arc::new(Mutex::new(0));

    let res = (0..num_tests)
        .into_par_iter()
        .try_for_each(|_| match run_one(None) {
            Ok(TestResult::Passed) | Ok(TestResult::Ignored) => {
                let mut num_finished = num_finished.lock().unwrap();
                *num_finished += 1;
                let percentage = (*num_finished as f32 / num_tests as f32) * 100.0;
                progress
                    .lock()
                    .unwrap()
                    .reach_percent(percentage.floor() as i32);
                Ok(())
            }
            Ok(fail) => Err(fail),
            Err(error) => Err(TestResult::Errored { error }),
        });

    progress.lock().unwrap().jobs_done();

    match res {
        Err(TestResult::Failed {
            seed,
            expected,
            actual,
        }) => {
            println!("test failed with seed {}\n", seed);
            println!("native: {}", String::from_utf8_lossy(&expected));
            println!("lucet-wasi: {}", String::from_utf8_lossy(&actual));
            std::process::exit(1)
        }
        Err(TestResult::Errored { error }) => println!("test errored: {}", error),
        Err(_) => unreachable!(),
        Ok(()) => println!("all tests passed"),
    }
}

fn gen_c<P: AsRef<Path>>(gen_c_path: P, seed: Seed) -> Result<(), Error> {
    Command::new("csmith")
        .arg("-s")
        .arg(format!("{}", seed))
        .arg("-o")
        .arg(gen_c_path.as_ref())
        .status()?;
    Ok(())
}

fn run_native<P: AsRef<Path>>(tmpdir: &TempDir, gen_c_path: P) -> Result<Option<Vec<u8>>, Error> {
    let gen_path = tmpdir.path().join("gen");

    Command::new("cc")
        .arg("-I/usr/include/csmith")
        .arg(gen_c_path.as_ref())
        .arg("-o")
        .arg(&gen_path)
        .output()?;

    let mut native_child = Command::new(&gen_path).stdout(Stdio::piped()).spawn()?;

    let exitcode = match native_child.wait_timeout(Duration::from_millis(1000))? {
        Some(status) => status.code(),
        None => {
            native_child.kill()?;
            native_child.wait()?.code()
        }
    };

    match exitcode {
        None => {
            // native code diverged or took too long, so was killed by the timeout
            return Ok(None);
        }
        Some(0) => (),
        Some(code) => {
            println!("native code returned non-zero exit code: {}", code);
            return Ok(None);
        }
    }

    let mut native_stdout = vec![];
    native_child
        .stdout
        .ok_or(format_err!("couldn't get stdout"))?
        .read_to_end(&mut native_stdout)?;

    Ok(Some(native_stdout))
}

enum TestResult {
    Passed,
    Ignored,
    Failed {
        seed: Seed,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    Errored {
        error: Error,
    },
}

fn run_one(seed: Option<Seed>) -> Result<TestResult, Error> {
    let tmpdir = TempDir::new().unwrap();

    let gen_c_path = tmpdir.path().join("gen.c");

    let seed = seed.unwrap_or(random::<Seed>());
    gen_c(&gen_c_path, seed)?;

    let native_stdout = if let Some(stdout) = run_native(&tmpdir, &gen_c_path)? {
        stdout
    } else {
        return Ok(TestResult::Ignored);
    };

    let (exitcode, wasm_stdout) = run_with_stdout(&tmpdir, &gen_c_path)?;

    assert_eq!(exitcode, 0);

    if &wasm_stdout != &native_stdout {
        Ok(TestResult::Failed {
            seed,
            expected: native_stdout,
            actual: wasm_stdout,
        })
    } else {
        Ok(TestResult::Passed)
    }
}

fn run_with_stdout<P: AsRef<Path>>(
    tmpdir: &TempDir,
    path: P,
) -> Result<(__wasi_exitcode_t, Vec<u8>), Error> {
    let ctx = WasiCtxBuilder::new().args(&["gen"]);

    let (pipe_out, pipe_in) = nix::unistd::pipe()?;

    let ctx = unsafe { ctx.raw_fd(1, pipe_in) }.build()?;

    let exitcode = run(tmpdir, path, ctx)?;

    let mut stdout_file = unsafe { File::from_raw_fd(pipe_out) };
    let mut stdout = vec![];
    stdout_file.read_to_end(&mut stdout)?;
    nix::unistd::close(stdout_file.into_raw_fd())?;

    Ok((exitcode, stdout))
}

fn run<P: AsRef<Path>>(
    tmpdir: &TempDir,
    path: P,
    ctx: WasiCtx,
) -> Result<__wasi_exitcode_t, Error> {
    let region = MmapRegion::create(1, &Limits::default())?;
    let module = wasi_test(tmpdir, path)?;

    let mut inst = region
        .new_instance_builder(module)
        .with_embed_ctx(ctx)
        .build()?;

    match inst.run(b"_start", &[]) {
        // normal termination implies 0 exit code
        Ok(_) => Ok(0),
        Err(lucet_runtime::Error::RuntimeTerminated(
            lucet_runtime::TerminationDetails::Provided(any),
        )) => Ok(*any
            .downcast_ref::<__wasi_exitcode_t>()
            .expect("termination yields an exitcode")),
        Err(e) => bail!("runtime error: {}", e),
    }
}

fn wasi_test<P: AsRef<Path>>(tmpdir: &TempDir, c_file: P) -> Result<Arc<dyn Module>, Error> {
    let wasm_build = Link::new(&[c_file]).cflag("-I/usr/include/csmith");

    let wasm_file = tmpdir.path().join("out.wasm");

    wasm_build.link(wasm_file.clone())?;

    let bindings = Bindings::from_file(
        Path::new(LUCET_WASI_FUZZ_ROOT)
            .parent()
            .unwrap()
            .join("lucet-wasi")
            .join("bindings.json"),
    )?;

    let native_build = Lucetc::new(wasm_file)?.bindings(bindings)?;

    let so_file = tmpdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule as Arc<dyn Module>)
}
