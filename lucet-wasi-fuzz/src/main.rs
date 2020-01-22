#![deny(bare_trait_objects)]

use anyhow::{bail, format_err, Error};
use libc::c_ulong;
use lucet_module::bindings::Bindings;
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::{WasiCtx, WasiCtxBuilder, __wasi_exitcode_t};
use lucet_wasi_sdk::{CompileOpts, Link};
use lucetc::{Lucetc, LucetcOpts};
use rand::prelude::random;
use rayon::prelude::*;
use regex::Regex;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::prelude::{FromRawFd, IntoRawFd, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;
use tempfile::TempDir;
use wait_timeout::ChildExt;

const LUCET_WASI_FUZZ_ROOT: &'static str = env!("CARGO_MANIFEST_DIR");

type Seed = c_ulong;

#[derive(StructOpt, Debug)]
#[structopt(name = "lucet-wasi-fuzz")]
enum Config {
    /// Test the Lucet toolchain against native code execution using Csmith
    #[structopt(name = "fuzz")]
    Fuzz {
        #[structopt(short = "n", long = "num-tests", default_value = "100")]
        /// The number of tests to run
        num_tests: usize,
    },

    /// Reduce a test case, starting from the given Csmith seed
    #[structopt(name = "creduce")]
    Creduce { seed: Seed },

    /// Creduce interestingness check (probably not useful directly)
    #[structopt(name = "creduce-interesting")]
    CreduceInteresting { creduce_src: PathBuf },

    /// Run a test case with the given Csmith seed
    #[structopt(name = "test-seed")]
    TestSeed { seed: Seed },
}

fn main() {
    lucet_runtime::lucet_internal_ensure_linked();
    lucet_wasi::export_wasi_funcs();

    match Config::from_args() {
        Config::Fuzz { num_tests } => run_many(num_tests),
        Config::Creduce { seed } => run_creduce_driver(seed),
        Config::CreduceInteresting { creduce_src } => run_creduce_interestingness(creduce_src),
        Config::TestSeed { seed } => run_one_seed(seed),
    }
}

fn run_creduce_driver(seed: Seed) {
    let tmpdir = TempDir::new().unwrap();

    // make the driver script

    let mut script = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .mode(0o777)
        .open(tmpdir.path().join("script.sh"))
        .unwrap();

    let current_exe = std::env::current_exe().unwrap();

    write!(
        script,
        "{}",
        format!(
            "#!/usr/bin/env sh\n{} creduce-interesting gen.c",
            current_exe.display()
        ),
    )
    .unwrap();

    drop(script);

    // reproduce the generated program, and then preprocess it

    let st = Command::new("csmith")
        .arg("-s")
        .arg(format!("{}", seed))
        .arg("-o")
        .arg(tmpdir.path().join("gen-original.c"))
        .status()
        .unwrap();
    assert!(st.success());

    let st = Command::new(host_clang())
        .arg("-I/usr/include/csmith")
        .arg("-m32")
        .arg("-E")
        .arg("-P")
        .arg(tmpdir.path().join("gen-original.c"))
        .arg("-o")
        .arg(tmpdir.path().join("gen.c"))
        .status()
        .unwrap();
    assert!(st.success());

    let st = Command::new("creduce")
        .current_dir(tmpdir.path())
        .arg("--n")
        .arg(format!("{}", std::cmp::max(1, num_cpus::get() - 1)))
        .arg("script.sh")
        .arg("gen.c")
        .status()
        .unwrap();
    assert!(st.success());

    print!(
        "{}",
        std::fs::read_to_string(tmpdir.path().join("gen.c")).unwrap()
    );
}

fn run_creduce_interestingness<P: AsRef<Path>>(src: P) {
    let tmpdir = TempDir::new().unwrap();

    match run_both(&tmpdir, src, None) {
        Ok(TestResult::Passed) => println!("test passed"),
        Ok(TestResult::Ignored) => println!("native build/execution failed"),
        Ok(TestResult::Failed {
            expected, actual, ..
        }) => {
            println!("test failed:\n");
            let expected = String::from_utf8_lossy(&expected);
            let actual = String::from_utf8_lossy(&actual);
            println!("native: {}", &expected);
            println!("lucet-wasi: {}", &actual);

            let re = Regex::new(r"^checksum = ([[:xdigit:]]{8})").unwrap();

            // a coarse way to stop creduce from producing degenerate cases that happen to yield
            // different output

            let expected_checksum = if let Some(caps) = re.captures(&expected) {
                if let Some(cap) = caps.get(1) {
                    cap.as_str().to_owned()
                } else {
                    // not interesting: no checksum captured from native output
                    exit(1);
                }
            } else {
                // not interesting: no checksum captured from native output
                exit(1);
            };

            let actual_checksum = if let Some(caps) = re.captures(&actual) {
                if let Some(cap) = caps.get(1) {
                    cap.as_str().to_owned()
                } else {
                    // interesting: checksum captured from native output but not wasm
                    exit(0)
                }
            } else {
                // interesting: checksum captured from native output but not wasm
                exit(0)
            };

            if expected_checksum == actual_checksum {
                // they match; not interesting
                exit(1);
            } else {
                exit(0);
            }
        }
        Ok(TestResult::Errored { error }) | Err(error) => println!("test errored: {}", error),
    }
    exit(1);
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
            exit(1);
        }
        Ok(TestResult::Errored { error }) | Err(error) => {
            println!("test errored: {}", error);
            exit(1);
        }
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
            println!("test failed with seed {}\n", seed.unwrap());
            println!("native: {}", String::from_utf8_lossy(&expected));
            println!("lucet-wasi: {}", String::from_utf8_lossy(&actual));
            exit(1);
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

fn run_both<P: AsRef<Path>>(
    tmpdir: &TempDir,
    src: P,
    seed: Option<Seed>,
) -> Result<TestResult, Error> {
    let native_stdout = if let Some(stdout) = run_native(&tmpdir, src.as_ref())? {
        stdout
    } else {
        return Ok(TestResult::Ignored);
    };

    let (exitcode, wasm_stdout) = run_with_stdout(&tmpdir, src.as_ref())?;

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

fn run_native<P: AsRef<Path>>(tmpdir: &TempDir, gen_c_path: P) -> Result<Option<Vec<u8>>, Error> {
    let gen_path = tmpdir.path().join("gen");

    let mut cmd = Command::new(host_clang());
    cmd.arg("-m32")
        .arg("-std=c11")
        .arg("-Werror=format")
        .arg("-Werror=uninitialized")
        .arg("-Werror=conditional-uninitialized")
        .arg("-I/usr/include/csmith")
        .arg(gen_c_path.as_ref())
        .arg("-o")
        .arg(&gen_path);
    if let Ok(flags) = std::env::var("HOST_CLANG_FLAGS") {
        cmd.args(flags.split_whitespace());
    }
    let res = cmd.output()?;

    if !res.status.success() {
        bail!(
            "native C compilation failed: {}",
            String::from_utf8_lossy(&res.stderr)
        );
    }

    if String::from_utf8_lossy(&res.stderr).contains("too few arguments in call") {
        bail!("saw \"too few arguments in call\" warning");
    }

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
        seed: Option<Seed>,
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

    run_both(&tmpdir, &gen_c_path, Some(seed))
}

fn run_with_stdout<P: AsRef<Path>>(
    tmpdir: &TempDir,
    path: P,
) -> Result<(__wasi_exitcode_t, Vec<u8>), Error> {
    let ctx = WasiCtxBuilder::new().args(&["gen"]);

    let (pipe_out, pipe_in) = nix::unistd::pipe()?;

    let ctx = ctx.stdout(unsafe { File::from_raw_fd(pipe_in) }).build()?;

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

    match inst.run("_start", &[]) {
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
    let wasm_build = Link::new(&[c_file]).with_cflag("-I/usr/include/csmith");

    let wasm_file = tmpdir.path().join("out.wasm");

    wasm_build.link(wasm_file.clone())?;

    let bindings = Bindings::from_file(
        Path::new(LUCET_WASI_FUZZ_ROOT)
            .parent()
            .unwrap()
            .join("lucet-wasi")
            .join("bindings.json"),
    )?;

    let native_build = Lucetc::new(wasm_file).with_bindings(bindings);

    let so_file = tmpdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule as Arc<dyn Module>)
}

fn host_clang() -> PathBuf {
    match std::env::var("HOST_CLANG") {
        Ok(clang) => PathBuf::from(clang),
        Err(_) => PathBuf::from("clang"),
    }
}
