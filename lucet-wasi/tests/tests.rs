mod test_helpers;

use crate::test_helpers::{
    lucet_wasi_tests_internal_ensure_linked, run, run_with_null_stdin, run_with_stdout,
    LUCET_WASI_ROOT,
};
use lucet_wasi::{WasiCtx, WasiCtxBuilder};
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn double_import() {
    lucet_wasi_tests_internal_ensure_linked();

    let mut ctx = WasiCtxBuilder::new();

    let (exitcode, stdout) = run_with_stdout("duplicate_import.wat", &mut ctx).unwrap();

    assert_eq!(stdout, "duplicate import works!\n");
    assert_eq!(exitcode, 0);
}

#[test]
fn hello() {
    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["hello"].iter());

    let (exitcode, stdout) = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        &mut ctx,
    )
    .unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello, wasi!\n");
}

#[test]
fn hello_args() {
    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["hello", "test suite"].iter());

    let (exitcode, stdout) = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        &mut ctx,
    )
    .unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello, test suite!\n");
}

#[test]
fn hello_env() {
    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["hello", "test suite"].iter());
    ctx.env("GREETING", "goodbye");

    let (exitcode, stdout) = run_with_stdout(
        Path::new(LUCET_WASI_ROOT).join("examples").join("hello.c"),
        &mut ctx,
    )
    .unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "goodbye, test suite!\n");
}

#[test]
fn exitcode() {
    let ctx = WasiCtx::new(["exitcode"].iter()).unwrap();

    let exitcode = run("exitcode.c", ctx).unwrap();

    assert_eq!(exitcode, 120);
}

#[test]
fn clock_getres() {
    let ctx = WasiCtx::new(["clock_getres"].iter()).unwrap();

    let exitcode = run("clock_getres.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getrusage() {
    let ctx = WasiCtx::new(["getrusage"].iter()).unwrap();

    let exitcode = run("getrusage.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn gettimeofday() {
    let ctx = WasiCtx::new(["gettimeofday"].iter()).unwrap();

    let exitcode = run("gettimeofday.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn getentropy() {
    let ctx = WasiCtx::new(["getentropy"].iter()).unwrap();

    let exitcode = run("getentropy.c", ctx).unwrap();

    assert_eq!(exitcode, 0);
}

#[test]
fn stdin() {
    use std::io::Write;
    use std::os::unix::io::FromRawFd;

    let (pipe_out, pipe_in) = nix::unistd::pipe().expect("can create pipe");

    let mut stdin_file = unsafe { File::from_raw_fd(pipe_in) };
    write!(stdin_file, "hello from stdin!").expect("pipe write succeeds");
    drop(stdin_file);

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["stdin"].iter());
    ctx.stdin(unsafe { File::from_raw_fd(pipe_out) });

    let (exitcode, stdout) = run_with_stdout("stdin.c", &mut ctx).unwrap();

    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, "hello from stdin!");
}

#[test]
fn preopen_populates() {
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();
    let preopen_dir = File::open(preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["preopen_populates"].iter());
    ctx.preopened_dir(preopen_dir, "/preopen");
    let ctx = ctx.build().expect("can build WasiCtx");

    let exitcode = run("preopen_populates.c", ctx).unwrap();

    drop(tmpdir);

    assert_eq!(exitcode, 0);
}

#[test]
fn write_file() {
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();
    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["write_file"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().expect("can build WasiCtx");

    let exitcode = run("write_file.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    let output = std::fs::read(preopen_host_path.join("output.txt")).unwrap();

    assert_eq!(output.as_slice(), b"hello, file!");

    drop(tmpdir);
}

#[test]
fn read_file() {
    const MESSAGE: &str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(preopen_host_path.join("input.txt"), MESSAGE).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["read_file"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");

    let (exitcode, stdout) = run_with_stdout("read_file.c", &mut ctx).unwrap();
    assert_eq!(exitcode, 0);

    assert_eq!(&stdout, MESSAGE);

    drop(tmpdir);
}

#[test]
fn read_file_twice() {
    const MESSAGE: &str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(preopen_host_path.join("input.txt"), MESSAGE).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["read_file_twice"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");

    let (exitcode, stdout) = run_with_stdout("read_file_twice.c", &mut ctx).unwrap();
    assert_eq!(exitcode, 0);

    let double_message = format!("{}{}", MESSAGE, MESSAGE);
    assert_eq!(stdout, double_message);

    drop(tmpdir);
}

#[test]
fn cant_dotdot() {
    const MESSAGE: &str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(
        preopen_host_path.parent().unwrap().join("outside.txt"),
        MESSAGE,
    )
    .unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["cant_dotdot"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().unwrap();

    let exitcode = run("cant_dotdot.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[ignore] // needs fd_readdir
#[test]
fn notdir() {
    const MESSAGE: &str = "hello from file!";
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();

    std::fs::write(preopen_host_path.join("notadir"), MESSAGE).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["notdir"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().unwrap();

    let exitcode = run("notdir.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn follow_symlink() {
    const MESSAGE: &str = "hello from file!";

    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    let subdir1 = preopen_host_path.join("subdir1");
    let subdir2 = preopen_host_path.join("subdir2");
    std::fs::create_dir_all(&subdir1).unwrap();
    std::fs::create_dir_all(&subdir2).unwrap();

    std::fs::write(subdir1.join("input.txt"), MESSAGE).unwrap();

    std::os::unix::fs::symlink("../subdir1/input.txt", subdir2.join("input_link.txt")).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["follow_symlink"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");

    let (exitcode, stdout) = run_with_stdout("follow_symlink.c", &mut ctx).unwrap();
    assert_eq!(exitcode, 0);
    assert_eq!(&stdout, MESSAGE);

    drop(tmpdir);
}

#[test]
fn symlink_loop() {
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    let subdir1 = preopen_host_path.join("subdir1");
    let subdir2 = preopen_host_path.join("subdir2");
    std::fs::create_dir_all(&subdir1).unwrap();
    std::fs::create_dir_all(&subdir2).unwrap();

    std::os::unix::fs::symlink("../subdir1/loop1", subdir2.join("loop2")).unwrap();
    std::os::unix::fs::symlink("../subdir2/loop2", subdir1.join("loop1")).unwrap();

    let preopen_dir = File::open(&preopen_host_path).unwrap();

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["symlink_loop"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().unwrap();

    let exitcode = run("symlink_loop.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn symlink_escape() {
    const MESSAGE: &str = "hello from file!";

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

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["symlink_escape"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().unwrap();

    let exitcode = run("symlink_escape.c", ctx).unwrap();
    assert_eq!(exitcode, 0);

    drop(tmpdir);
}

#[test]
fn pseudoquine() {
    let examples_dir = Path::new(LUCET_WASI_ROOT).join("examples");
    let pseudoquine_c = examples_dir.join("pseudoquine.c");

    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["pseudoquine"].iter());
    ctx.preopened_dir(File::open(examples_dir).unwrap(), "/examples");

    let (exitcode, stdout) = run_with_stdout(&pseudoquine_c, &mut ctx).unwrap();

    assert_eq!(exitcode, 0);

    let expected = std::fs::read_to_string(&pseudoquine_c).unwrap();

    assert_eq!(stdout, expected);
}

// ACF 2019-10-03: temporarily disabled until we figure out why it's behaving differently only in
// one CI environment
#[ignore]
#[test]
fn poll() {
    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["poll"].iter());
    let exitcode = run_with_null_stdin("poll.c", &mut ctx).unwrap();
    assert_eq!(exitcode, 0);
}

#[test]
fn stat() {
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();
    let preopen_dir = File::open(&preopen_host_path).unwrap();
    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["stat"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().expect("can build WasiCtx");
    let exitcode = run("stat.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}

#[test]
fn fs() {
    let tmpdir = TempDir::new().unwrap();
    let preopen_host_path = tmpdir.path().join("preopen");
    std::fs::create_dir(&preopen_host_path).unwrap();
    let preopen_dir = File::open(&preopen_host_path).unwrap();
    let mut ctx = WasiCtxBuilder::new();
    ctx.args(["stat"].iter());
    ctx.preopened_dir(preopen_dir, "/sandbox");
    let ctx = ctx.build().expect("can build WasiCtx");
    let exitcode = run("fs.c", ctx).unwrap();
    assert_eq!(exitcode, 0);
}
