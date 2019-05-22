use failure::{format_err, Error};
use log::info;
use lucet_idl::{self, Backend, Config, Package};
use lucet_wasi;
use lucetc::{Lucetc, LucetcOpts};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub struct WasiProject {
    tempdir: TempDir,
    package: Package,
}

impl WasiProject {
    pub fn new(package: Package) -> Self {
        let tempdir = TempDir::new().expect("create tempdir for WasiProject");
        Self { tempdir, package }
    }

    fn context(&self, name: &str) -> PathBuf {
        let dir = self.tempdir.path().join(name);
        fs::create_dir(dir.clone()).expect("create context subdir");
        dir
    }

    fn create_rust_guest_source(&self, dir: &Path) -> Result<PathBuf, Error> {
        let f = dir.join("idl.rs");
        info!("running lucet_idl::codegen for rust guest...");
        lucet_idl::codegen(
            &self.package,
            &Config {
                backend: Backend::RustGuest,
            },
            Box::new(File::create(f.clone()).expect("create rust guest idl")),
        )?;
        info!("done!");
        Ok(f)
    }

    pub fn compile_rust_guest(&self) -> Result<PathBuf, Error> {
        let dir = self.context("rust_guest_source");
        let wasm_file = self.context("compile_rust_guest").join("out.wasm");
        let rust_guest_source = self.create_rust_guest_source(&dir)?;

        info!("creating main.rs for rust guest...");
        let main_path = dir.join("main.rs");
        let mut main_file = File::create(&main_path)?;
        main_file.write_all(
            b"
#[allow(unused)]
mod idl;

fn main() {
    println!(\"hello, world\");
}",
        )?;

        info!("running rustc...");
        let cmd_rustc = Command::new("rustc")
            .arg("+nightly")
            .arg(main_path)
            .arg("--target=wasm32-wasi")
            .arg("--test")
            .arg("-o")
            .arg(wasm_file.clone())
            .status()
            .expect("run rustc");
        info!("done!");
        if !cmd_rustc.success() {
            Err(format_err!(""))?
        }
        Ok(wasm_file)
    }

    pub fn codegen_rust_guest(&self) -> Result<PathBuf, Error> {
        let dir = self.context("rust_guest_codegen");
        let so_file = dir.join("guest.so");
        let wasm_file = self.compile_rust_guest()?;
        let lucetc = Lucetc::new(&wasm_file).with_bindings(lucet_wasi::bindings());
        lucetc.shared_object_file(&so_file)?;
        Ok(so_file)
    }

    fn create_rust_host_idl(&self, dir: &Path) -> Result<PathBuf, Error> {
        let f = dir.join("idl.rs");
        info!("running lucet_idl::codegen for rust host...");
        lucet_idl::codegen(
            &self.package,
            &Config {
                backend: Backend::RustHost,
            },
            Box::new(File::create(f.clone()).expect("create rust host idl")),
        )?;
        info!("done!");
        Ok(f)
    }

    pub fn compile_rust_host(&self) -> Result<PathBuf, Error> {
        let dir = self.context("rust_host_source");
        let _ = self.create_rust_host_idl(&dir)?;

        info!("creating main.rs for rust host...");
        let mut main_file = File::create(dir.join("main.rs"))?;
        main_file.write_all(
            b"
#[allow(unused)]
mod idl;

fn main() {
    println!(\"hello, world\");
}",
        )?;

        info!("done!");
        unimplemented!()
    }
}
