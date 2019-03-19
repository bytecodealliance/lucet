mod test_helpers;

use crate::test_helpers::{run_with_stdout, LUCET_WASI_ROOT};
use failure::Error;
use lucet_wasi::WasiCtx;
use std::path::Path;

#[test]
fn hello() -> Result<(), Error> {
    let ctx = WasiCtx::new("hello", &[]);

    let stdout = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        ctx,
    )?;

    assert_eq!(&stdout, "hello, wasi!\n");

    Ok(())
}

#[test]
fn hello_args() -> Result<(), Error> {
    let ctx = WasiCtx::new("hello", &["test suite"]);

    let stdout = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        ctx,
    )?;

    assert_eq!(&stdout, "hello, test suite!\n");

    Ok(())
}

#[test]
fn hello_env() -> Result<(), Error> {
    let ctx = WasiCtx::new_with_env(
        "hello",
        &["test suite"],
        std::iter::once(("GREETING", "goodbye")),
    );

    let stdout = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        ctx,
    )?;

    assert_eq!(&stdout, "goodbye, test suite!\n");

    Ok(())
}
