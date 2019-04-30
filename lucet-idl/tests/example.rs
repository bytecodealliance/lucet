use lucet_idl;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn compile_and_run() {
    let mut source = String::new();
    File::open("tests/example.idl")
        .expect("open example.idl")
        .read_to_string(&mut source)
        .expect("read example.idl");

    let config = lucet_idl::Config {
        backend: lucet_idl::Backend::C,
        backend_config: lucet_idl::BackendConfig::default(),
        target: lucet_idl::Target::Generic,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    lucet_idl::run(
        &config,
        &source,
        File::create(tempdir.path().join("example.h")).expect("create file"),
    )
    .expect("run lucet_idl");

    let cmd_cc = Command::new("cc")
        .arg("--std=c99")
        .arg("-I")
        .arg(tempdir.path())
        .arg("tests/example_driver.c")
        .arg("-o")
        .arg(tempdir.path().join("example"))
        .status()
        .expect("run cc");
    assert!(cmd_cc.success(), "failure to compile generated code");

    let cmd_run = Command::new(tempdir.path().join("example"))
        .status()
        .expect("run generated code");
    assert!(cmd_run.success(), "failure to run generated code");
}
