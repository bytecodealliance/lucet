use lucet_idl;
use std::fs::{create_dir, File};
use std::io::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn compile_and_run_c() {
    let mut source = String::new();
    File::open("tests/example.idl")
        .expect("open example.idl")
        .read_to_string(&mut source)
        .expect("read example.idl");

    let config = lucet_idl::Config {
        backend: lucet_idl::Backend::C,
        target: lucet_idl::Target::Generic,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    lucet_idl::run(
        &config,
        &source,
        Box::new(File::create(tempdir.path().join("example.h")).expect("create file")),
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

#[test]
fn compile_and_run_rust() {
    let mut source = String::new();
    File::open("tests/example.idl")
        .expect("open example.idl")
        .read_to_string(&mut source)
        .expect("read example.idl");

    let config = lucet_idl::Config {
        backend: lucet_idl::Backend::Rust,
        target: lucet_idl::Target::Generic,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    create_dir(tempdir.path().join("src")).expect("create src");
    let gen_file = tempdir.path().join("src").join("lib.rs");

    let mut cargo_toml =
        File::create(tempdir.path().join("Cargo.toml")).expect("create cargo.toml");
    cargo_toml
        .write_all(
            "
[package]
name = \"test\"
version = \"0.1.0\"
edition = \"2018\"
[lib]
crate-type=[\"rlib\"]
[dependencies]
memoffset=\"*\""
                .as_bytes(),
        )
        .unwrap();
    drop(cargo_toml);

    lucet_idl::run(
        &config,
        &source,
        Box::new(File::create(gen_file.clone()).expect("create file")),
    )
    .expect("run lucet_idl");

    let cmd_rustc = Command::new("cargo")
        .arg("test")
        .current_dir(tempdir.path())
        .status()
        .expect("run rustcc");
    assert!(cmd_rustc.success(), "failure to compile generated code");
}
