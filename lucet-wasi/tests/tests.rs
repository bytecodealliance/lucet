mod test_helpers;
use crate::test_helpers::{
    init_tracing, lucet_wasi_tests_internal_ensure_linked, run, run_with_null_stdin,
    run_with_stdout, LUCET_WASI_ROOT,
};
use lucet_wasi::WasiCtxBuilder;
use std::io::{Read, Write};
use std::path::Path;

#[test]
fn double_import() {
    init_tracing();
    lucet_wasi_tests_internal_ensure_linked();

    let ctx = WasiCtxBuilder::new();

    let (exitcode, stdout) = run_with_stdout("duplicate_import.wat", ctx).unwrap();

    assert_eq!(stdout, "duplicate import works!\n");
    assert_eq!(exitcode, 0);
}

#[test]
fn hello() {
    init_tracing();
    let ctx = WasiCtxBuilder::new().args(&["hello".to_owned()]).unwrap();

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
    init_tracing();
    let mut ctx = WasiCtxBuilder::new();
    ctx = ctx
        .args(&["hello".to_owned(), "test suite".to_owned()])
        .unwrap();

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
    init_tracing();
    let mut ctx = WasiCtxBuilder::new();
    ctx = ctx
        .args(&["hello".to_owned(), "test suite".to_owned()])
        .unwrap();
    ctx = ctx.env("GREETING", "goodbye").unwrap();

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
    init_tracing();
    let ctx = WasiCtxBuilder::new()
        .args(&["exitcode".to_owned()])
        .unwrap()
        .build();

    let exitcode = run("exitcode.c", ctx).unwrap();

    assert_eq!(exitcode, 120);
}

#[test]
fn clock_getres() {
    init_tracing();
    let ctx = WasiCtxBuilder::new()
        .args(&["clock_getres".to_owned()])
        .unwrap();

    let (exitcode, _) = run_with_stdout("clock_getres.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn gettimeofday() {
    init_tracing();
    let ctx = WasiCtxBuilder::new()
        .args(&["gettimeofday".to_owned()])
        .unwrap()
        .build();

    let exitcode = run("gettimeofday.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getentropy() {
    init_tracing();
    let ctx = WasiCtxBuilder::new()
        .args(&["getentropy".to_owned()])
        .unwrap()
        .build();

    let exitcode = run("getentropy.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn stdin() {
    init_tracing();
    let stdin = wasi_common::pipe::ReadPipe::from("hello from stdin!");

    let ctx = WasiCtxBuilder::new()
        .args(&["stdin".to_owned()])
        .unwrap()
        .stdin(Box::new(stdin));

    let (exitcode, stdout) = run_with_stdout("stdin.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello from stdin!");
}

#[test]
fn preopen_populates() {
    init_tracing();
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["preopen_populates".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/preopen")
        .unwrap()
        .build();

    let exitcode = run("preopen_populates.c", ctx).unwrap();

    drop(tmpdir);

    assert_eq!(exitcode, 0);
}

#[test]
fn write_file() {
    init_tracing();
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["write_file".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap()
        .build();

    let exitcode = run("write_file.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    let mut output = String::new();
    tmpdir
        .open("preopen/output.txt")
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();

    assert_eq!(output.as_str(), "hello, file!");

    drop(tmpdir);
}

#[test]
fn read_file() {
    const MESSAGE: &str = "hello from file!";
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    preopen_dir
        .create("input.txt")
        .unwrap()
        .write_all(MESSAGE.as_bytes())
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["read_file".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap();

    let (exitcode, stdout) = run_with_stdout("read_file.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    assert_eq!(&stdout, MESSAGE);

    drop(tmpdir);
}

#[test]
fn read_file_twice() {
    init_tracing();
    const MESSAGE: &str = "hello from file!";
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    preopen_dir
        .create("input.txt")
        .unwrap()
        .write_all(MESSAGE.as_bytes())
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["read_file_twice".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap();

    let (exitcode, stdout) = run_with_stdout("read_file_twice.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    let double_message = format!("{}{}", MESSAGE, MESSAGE);
    assert_eq!(stdout, double_message);

    drop(tmpdir);
}

#[test]
fn cant_dotdot() {
    init_tracing();
    const MESSAGE: &str = "hello from file!";
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    tmpdir
        .create("outside.txt")
        .unwrap()
        .write_all(MESSAGE.as_bytes())
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["cant_dotdot".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap();

    let (exitcode, _) = run_with_stdout("cant_dotdot.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn notdir() {
    init_tracing();
    const MESSAGE: &str = "hello from file!";

    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    preopen_dir
        .create("notadir")
        .unwrap()
        .write_all(MESSAGE.as_bytes())
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["notdir".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap()
        .build();

    let exitcode = run("notdir.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn follow_symlink() {
    init_tracing();
    const MESSAGE: &str = "hello from file!";

    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    preopen_dir.create_dir("subdir1").unwrap();
    preopen_dir.create_dir("subdir2").unwrap();
    let subdir1 = preopen_dir.open_dir("subdir1").unwrap();
    let subdir2 = preopen_dir.open_dir("subdir2").unwrap();

    subdir1
        .create("input.txt")
        .unwrap()
        .write_all(MESSAGE.as_bytes())
        .unwrap();

    subdir2
        .symlink("../subdir1/input.txt", "input_link.txt")
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["follow_symlink".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap();

    let (exitcode, stdout) = run_with_stdout("follow_symlink.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, MESSAGE);

    drop(tmpdir);
}

#[test]
fn symlink_loop() {
    init_tracing();
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();
    preopen_dir.create_dir("subdir1").unwrap();
    preopen_dir.create_dir("subdir2").unwrap();

    preopen_dir
        .symlink("../subdir1/loop1", "subdir2/loop2")
        .unwrap();
    preopen_dir
        .symlink("../subdir2/loop2", "subdir1/loop1")
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["symlink_loop".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap()
        .build();

    let exitcode = run("symlink_loop.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn symlink_escape() {
    init_tracing();
    const MESSAGE: &str = "hello from file!";

    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();
    preopen_dir.create_dir("subdir").unwrap();

    tmpdir
        .create("outside.txt")
        .unwrap()
        .write_all(MESSAGE.as_bytes())
        .unwrap();
    preopen_dir
        .symlink("../../outside.txt", "subdir/outside.txt")
        .unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["symlink_escape".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap()
        .build();

    let exitcode = run("symlink_escape.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn pseudoquine() {
    init_tracing();
    let examples_dir = Path::new(LUCET_WASI_ROOT).join("examples");
    let pseudoquine_c = examples_dir.join("pseudoquine.c");

    let ctx = WasiCtxBuilder::new()
        .args(&["pseudoquine".to_owned()])
        .unwrap()
        .preopened_dir(
            unsafe { cap_std::fs::Dir::open_ambient_dir(examples_dir).unwrap() },
            "/examples",
        )
        .unwrap();

    let (exitcode, stdout) = run_with_stdout(&pseudoquine_c, ctx).unwrap();

    assert_eq!(exitcode, 0);

    let expected = std::fs::read_to_string(&pseudoquine_c).unwrap();

    assert_eq!(stdout, expected);
}

// ACF 2019-10-03: temporarily disabled until we figure out why it's behaving differently only in
// one CI environment
#[ignore]
#[test]
fn poll() {
    init_tracing();
    let ctx = WasiCtxBuilder::new().args(&["poll".to_owned()]).unwrap();
    let exitcode = run_with_null_stdin("poll.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}

#[test]
fn stat() {
    init_tracing();
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["stat".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap()
        .build();
    let exitcode = run("stat.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}

#[test]
fn fs() {
    init_tracing();
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["stat".to_owned()])
        .unwrap()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap();
    let (exitcode, _) = run_with_stdout("fs.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}

#[test]
fn readdir() {
    init_tracing();
    let tmpdir = unsafe { cap_tempfile::tempdir().unwrap() };
    tmpdir.create_dir("preopen").unwrap();
    let preopen_dir = tmpdir.open_dir("preopen").unwrap();

    let ctx = WasiCtxBuilder::new()
        .preopened_dir(preopen_dir, "/sandbox")
        .unwrap();
    let (exitcode, _) = run_with_stdout("readdir.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}
