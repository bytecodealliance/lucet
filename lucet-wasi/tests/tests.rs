mod test_helpers;

use crate::test_helpers::{run, run_with_stdout, LUCET_WASI_ROOT};
use lucet_wasi::{WasiCtx, WasiCtxBuilder};
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn hello() {
    io::stderr().write(b"hello\n");
    let ctx = WasiCtxBuilder::new().args(&["hello"]);

    io::stderr().write(b"hello built\n");
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
    io::stderr().write(b"hello_args\n");
    let ctx = WasiCtxBuilder::new().args(&["hello", "test suite"]);

    io::stderr().write(b"hello_args built\n");
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
    io::stderr().write(b"hello_env\n");
    let ctx = WasiCtxBuilder::new()
        .args(&["hello", "test suite"])
        .env("GREETING", "goodbye");

    io::stderr().write(b"hello_env\n");
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
    io::stderr().write(b"exitcode\n");
    let ctx = WasiCtx::new(&["exitcode"]);

    io::stderr().write(b"exitcode built\n");
    let exitcode = run("exitcode.c", ctx).unwrap();

    assert_eq!(exitcode, 120);
}

#[test]
fn clock_getres() {
    io::stderr().write(b"clock_getres\n");
    let ctx = WasiCtx::new(&["clock_getres"]);

    io::stderr().write(b"clock_getres built\n");
    let exitcode = run("clock_getres.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getrusage() {
    io::stderr().write(b"getrusage\n");
    let ctx = WasiCtx::new(&["getrusage"]);

    io::stderr().write(b"getrusage built\n");
    let exitcode = run("getrusage.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn gettimeofday() {
    io::stderr().write(b"gettimeofday\n");
    let ctx = WasiCtx::new(&["gettimeofday"]);

    io::stderr().write(b"gettimeofday built\n");
    let exitcode = run("gettimeofday.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getentropy() {
    io::stderr().write(b"getentropy\n");
    let ctx = WasiCtx::new(&["getentropy"]);

    io::stderr().write(b"getentropy built\n");
    let exitcode = run("getentropy.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn stdin() {
    io::stderr().write(b"stdin\n");
    use std::io::Write;
    use std::os::unix::io::FromRawFd;

    let (pipe_out, pipe_in) = nix::unistd::pipe().expect("can create pipe");

    let mut stdin_file = unsafe { File::from_raw_fd(pipe_in) };
    write!(stdin_file, "hello from stdin!").expect("pipe write succeeds");
    drop(stdin_file);

    let ctx = unsafe { WasiCtxBuilder::new().args(&["stdin"]).raw_fd(0, pipe_out) };
    io::stderr().write(b"stdin built\n");

    let (exitcode, stdout) = run_with_stdout("stdin.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello from stdin!");
}

#[test]
fn preopen_populates() {
    io::stderr().write(b"preopen_populates\n");
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();
    let preopen_dir = File::open(preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["preopen_populates"])
        .preopened_dir(preopen_dir, "/preopen")
        .build()
        .expect("can build WasiCtx");

    io::stderr().write(b"preopen_populates built\n");
    let exitcode = run("preopen_populates.c", ctx).unwrap();

    drop(tmpdir);

    assert_eq!(exitcode, 0);
}

#[test]
fn write_file() {
    io::stderr().write(b"write_file\n");
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();
    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["write_file"])
        .preopened_dir(preopen_dir, "/sandbox")
        .build()
        .expect("can build WasiCtx");

    io::stderr().write(b"write_file built\n");
    let exitcode = run("write_file.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    let output = std::fs::read(preopen_host_path.join("output.txt")).unwrap();

    assert_eq!(output.as_slice(), b"hello, file!");

    drop(tmpdir);
}

#[test]
#[ignore]
fn read_file() {
    io::stderr().write(b"read_file\n");
    const MESSAGE: &'static str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(preopen_host_path.join("input.txt"), MESSAGE).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["read_file"])
        .preopened_dir(preopen_dir, "/sandbox");
    io::stderr().write(b"read_file built\n");

    let (exitcode, stdout) = run_with_stdout("read_file.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    assert_eq!(&stdout, MESSAGE);

    drop(tmpdir);
}

#[test]
fn read_file_twice() {
    io::stderr().write(b"read_file_twice\n");
    const MESSAGE: &'static str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(preopen_host_path.join("input.txt"), MESSAGE).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["read_file_twice"])
        .preopened_dir(preopen_dir, "/sandbox");
    io::stderr().write(b"read_file_twice built\n");

    let (exitcode, stdout) = run_with_stdout("read_file_twice.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    let double_message = format!("{}{}", MESSAGE, MESSAGE);
    assert_eq!(stdout, double_message);

    drop(tmpdir);
}

#[test]
fn cant_dotdot() {
    io::stderr().write(b"cant_dotdot\n");
    const MESSAGE: &'static str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(
        preopen_host_path.parent().unwrap().join("outside.txt"),
        MESSAGE,
    )
    .unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    io::stderr().write(b"cant_dotdot built\n");
    let ctx = WasiCtxBuilder::new()
        .args(&["cant_dotdot"])
        .preopened_dir(preopen_dir, "/sandbox")
        .build()
        .unwrap();

    let exitcode = run("cant_dotdot.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[ignore] // needs fd_readdir
#[test]
fn notdir() {
    io::stderr().write(b"notdir\n");
    const MESSAGE: &'static str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(preopen_host_path.join("notadir"), MESSAGE).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["notdir"])
        .preopened_dir(preopen_dir, "/sandbox")
        .build()
        .unwrap();

    io::stderr().write(b"notdir built\n");
    let exitcode = run("notdir.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn follow_symlink() {
    io::stderr().write(b"follow_symlink\n");
    const MESSAGE: &'static str = "hello from file!";

    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    let subdir1 = preopen_host_path.join("subdir1");
    let subdir2 = preopen_host_path.join("subdir2");
    std::fs::create_dir_all(&subdir1).unwrap();
    std::fs::create_dir_all(&subdir2).unwrap();

    std::fs::write(subdir1.join("input.txt"), MESSAGE).unwrap();

    std::os::unix::fs::symlink("../subdir1/input.txt", subdir2.join("input_link.txt")).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["follow_symlink"])
        .preopened_dir(preopen_dir, "/sandbox");

    io::stderr().write(b"follow_symlink built\n");
    let (exitcode, stdout) = run_with_stdout("follow_symlink.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, MESSAGE);

    drop(tmpdir);
}

#[test]
fn symlink_loop() {
    io::stderr().write(b"symlink_loop\n");
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    let subdir1 = preopen_host_path.join("subdir1");
    let subdir2 = preopen_host_path.join("subdir2");
    std::fs::create_dir_all(&subdir1).unwrap();
    std::fs::create_dir_all(&subdir2).unwrap();

    std::os::unix::fs::symlink("../subdir1/loop1", subdir2.join("loop2")).unwrap();
    std::os::unix::fs::symlink("../subdir2/loop2", subdir1.join("loop1")).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["symlink_loop"])
        .preopened_dir(preopen_dir, "/sandbox")
        .build()
        .unwrap();

    io::stderr().write(b"symlink_loop built\n");
    let exitcode = run("symlink_loop.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn symlink_escape() {
    io::stderr().write(b"symlink_escape\n");
    const MESSAGE: &'static str = "hello from file!";

    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    let subdir = preopen_host_path.join("subdir");
    std::fs::create_dir_all(&subdir).unwrap();

    std::fs::write(
        preopen_host_path.parent().unwrap().join("outside.txt"),
        MESSAGE,
    )
    .unwrap();
    std::os::unix::fs::symlink("../../outside.txt", subdir.join("outside.txt")).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let ctx = WasiCtxBuilder::new()
        .args(&["symlink_escape"])
        .preopened_dir(preopen_dir, "/sandbox")
        .build()
        .unwrap();

    io::stderr().write(b"symlink_escape built\n");
    let exitcode = run("symlink_escape.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn pseudoquine() {
    io::stderr().write(b"pseudoquine\n");
    let examples_dir = Path::new(LUCET_WASI_ROOT).join("examples");
    let pseudoquine_c = examples_dir.join("pseudoquine.c");

    let ctx = WasiCtxBuilder::new()
        .args(&["pseudoquine"])
        .preopened_dir(File::open(examples_dir).unwrap(), "/examples");

    io::stderr().write(b"pseudoquine built\n");
    let (exitcode, stdout) = run_with_stdout(&pseudoquine_c, ctx).unwrap();

    assert_eq!(exitcode, 0);

    let expected = std::fs::read_to_string(&pseudoquine_c).unwrap();

    assert_eq!(stdout, expected);
}

#[test]
fn poll() {
    io::stderr().write(b"poll\n");
    let ctx = WasiCtxBuilder::new().args(&["poll"]).build().unwrap();
    io::stderr().write(b"poll built\n");
    let exitcode = run("poll.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}
