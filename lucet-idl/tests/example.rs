use lucet_idl;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn compile_c_guest() {
    let config = lucet_idl::Config {
        backend: lucet_idl::Backend::CGuest,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    lucet_idl::run(
        &config,
        Path::new("tests/example.idl"),
        Box::new(File::create(tempdir.path().join("example.h")).expect("create file")),
    )
    .expect("run lucet_idl");

    let cmd_cc = Command::new("/opt/wasi-sdk/bin/clang")
        .arg("--target=wasm32-wasi")
        .arg("--std=c99")
        .arg("-Wl,--allow-undefined")
        .arg("-I")
        .arg(tempdir.path())
        .arg("tests/example_driver.c")
        .arg("-o")
        .arg(tempdir.path().join("example"))
        .status()
        .expect("run cc");
    assert!(cmd_cc.success(), "failure to compile generated code");
}

#[test]
fn compile_and_test_rust_guest() {
    compile_and_test_rust(lucet_idl::Backend::RustGuest)
}

/* DISABLED: host needs the lucet_hostcalls! macro from lucet_runtime,
 * and we dont want to manage the dep here, lucet-idl-test can handle it
#[test]
fn compile_and_test_rust_host() {
    compile_and_test_rust(lucet_idl::Backend::RustHost)
}
*/

fn compile_and_test_rust(backend: lucet_idl::Backend) {
    let config = lucet_idl::Config { backend };

    let tempdir = TempDir::new().expect("create tempdir");

    let gen_file = tempdir.path().join("out.rs");

    lucet_idl::run(
        &config,
        Path::new("tests/example.idl"),
        Box::new(File::create(gen_file.clone()).expect("create file")),
    )
    .expect("run lucet_idl");

    let cmd_rustc = Command::new("rustc")
        .arg(gen_file.clone())
        .arg("--test")
        .arg("--allow=dead_code")
        .arg("-o")
        .arg(tempdir.path().join("example"))
        .status()
        .expect("run rustcc");
    assert!(cmd_rustc.success(), "failure to compile generated code");

    let cmd_run = Command::new(tempdir.path().join("example"))
        .status()
        .expect("run generated code");
    assert!(cmd_run.success(), "failure to run generated code");
}
