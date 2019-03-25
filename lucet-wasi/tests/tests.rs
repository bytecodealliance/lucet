mod test_helpers;

use crate::test_helpers::{run, run_with_stdout, LUCET_WASI_ROOT};
use lucet_wasi::{WasiCtx, WasiCtxBuilder};
use std::path::Path;

#[test]
fn hello() {
    let ctx = WasiCtxBuilder::new().args(&["hello"]);

    let (exitcode, stdout) = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        ctx,
    )
    .unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello, wasi!\n");
}

#[test]
fn hello_args() {
    let ctx = WasiCtxBuilder::new().args(&["hello", "test suite"]);

    let (exitcode, stdout) = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        ctx,
    )
    .unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello, test suite!\n");
}

#[test]
fn hello_env() {
    let ctx = WasiCtxBuilder::new()
        .args(&["hello", "test suite"])
        .env("GREETING", "goodbye");

    let (exitcode, stdout) = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        ctx,
    )
    .unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "goodbye, test suite!\n");
}

#[test]
fn exitcode() {
    let ctx = WasiCtx::new(&["exitcode"]);

    let exitcode = run("exitcode.c", ctx).unwrap();

    assert_eq!(exitcode, 120);
}

#[test]
fn clock_getres() {
    let ctx = WasiCtx::new(&["clock_getres"]);

    let exitcode = run("clock_getres.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getrusage() {
    let ctx = WasiCtx::new(&["getrusage"]);

    let exitcode = run("getrusage.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn gettimeofday() {
    let ctx = WasiCtx::new(&["gettimeofday"]);

    let exitcode = run("gettimeofday.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getentropy() {
    let ctx = WasiCtx::new(&["getentropy"]);

    let exitcode = run("getentropy.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn stdin() {
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::io::FromRawFd;

    let (pipe_out, pipe_in) = nix::unistd::pipe().expect("can create pipe");

    let mut stdin_file = unsafe { File::from_raw_fd(pipe_in) };
    write!(stdin_file, "hello from stdin!").expect("pipe write succeeds");
    drop(stdin_file);

    let ctx = unsafe { WasiCtxBuilder::new().args(&["stdin"]).raw_fd(0, pipe_out) };

    let (exitcode, stdout) = run_with_stdout("stdin.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello from stdin!");
}
