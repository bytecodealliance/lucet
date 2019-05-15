use lucet_idl::{codegen, Backend, Config, Package, Target};
use std::fs::{create_dir, File};
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

pub fn rust_codegen(package: &Package) {
    let config = Config {
        backend: Backend::Rust,
        target: Target::Generic,
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
    codegen(
        package,
        &config,
        Box::new(File::create(gen_file.clone()).expect("create file")),
    )
    .expect("lucet_idl codegen");

    let cmd_rustc = Command::new("cargo")
        .arg("test")
        .current_dir(tempdir.path())
        .status()
        .expect("run cargo test");

    if !cmd_rustc.success() {
        Command::new("cat")
            .arg(gen_file.clone())
            .status()
            .expect("debug output");
    }
    assert!(cmd_rustc.success(), "failure to compile generated code");
}

pub fn c_codegen(package: &Package) {
    let config = lucet_idl::Config {
        backend: lucet_idl::Backend::C,
        target: lucet_idl::Target::Generic,
    };

    let tempdir = TempDir::new().expect("create tempdir");

    codegen(
        package,
        &config,
        Box::new(File::create(tempdir.path().join("example.c")).expect("create file")),
    )
    .expect("lucet_idl codegen");

    let cmd_cc = Command::new("cc")
        .arg("--std=c99")
        .arg("-c")
        .arg(tempdir.path().join("example.c"))
        .arg("-o")
        .arg(tempdir.path().join("example.o"))
        .status()
        .expect("run cc");

    if !cmd_cc.success() {
        Command::new("cat")
            .arg(tempdir.path().join("example.c"))
            .status()
            .expect("debug output");
    }
    assert!(cmd_cc.success(), "failure to compile generated code");

    /*
    let cmd_run = Command::new(tempdir.path().join("example"))
        .status()
        .expect("run generated code");
    assert!(cmd_run.success(), "failure to run generated code");
    */
}
