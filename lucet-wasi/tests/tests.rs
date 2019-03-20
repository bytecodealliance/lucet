mod test_helpers;

use crate::test_helpers::{run, run_with_stdout, LUCET_WASI_ROOT};
use lucet_wasi::WasiCtx;
use std::path::Path;

#[test]
fn hello() {
    let ctx = WasiCtx::new("hello", &[]);

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
    let ctx = WasiCtx::new("hello", &["test suite"]);

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
    let ctx = WasiCtx::new_with_env(
        "hello",
        &["test suite"],
        std::iter::once(("GREETING", "goodbye")),
    );

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
    let ctx = WasiCtx::new("exitcode", &[]);

    let exitcode = run("exitcode.c", ctx).unwrap();

    assert_eq!(exitcode, 120);
}

#[test]
fn clock_getres() {
    let ctx = WasiCtx::new("clock_getres", &[]);

    let exitcode = run("clock_getres.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getrusage() {
    let ctx = WasiCtx::new("getrusage", &[]);

    let exitcode = run("getrusage.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn gettimeofday() {
    let ctx = WasiCtx::new("gettimeofday", &[]);

    let exitcode = run("gettimeofday.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getentropy() {
    let ctx = WasiCtx::new("getentropy", &[]);

    let exitcode = run("getentropy.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}
