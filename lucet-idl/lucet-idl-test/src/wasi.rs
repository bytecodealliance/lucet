use failure::{format_err, Error};
use fs2::FileExt;
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
        let _rust_guest_source = self.create_rust_guest_source(&dir)?;

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
            .arg("-o")
            .arg(wasm_file.clone())
            .status()
            .expect("run rustc");
        info!("done!");
        if !cmd_rustc.success() {
            Err(format_err!("rustc error building guest"))?
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

    fn create_rust_host_idl(&self, f: File) -> Result<(), Error> {
        info!("running lucet_idl::codegen for rust host...");
        lucet_idl::codegen(
            &self.package,
            &Config {
                backend: Backend::RustHost,
            },
            Box::new(f),
        )?;
        info!("done!");
        Ok(())
    }

    pub fn create_rust_host(&self) -> Result<HostApp, Error> {
        let resource_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("rust_host");
        let mut host_app = HostApp::new(resource_dir)?;
        let idl_file = host_app.source_file("idl.rs")?;
        self.create_rust_host_idl(idl_file)?;
        Ok(host_app)
    }
}

pub struct HostApp {
    root: PathBuf,
    lockfile: File,
    tempdir: TempDir,
    backups: Vec<(PathBuf, PathBuf)>,
}

impl HostApp {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, Error> {
        let lockfile_path = root.as_ref().join(".rust_host.lock");
        if !lockfile_path.exists() {
            File::create(&lockfile_path)?;
        }

        let lockfile = File::open(lockfile_path)?;

        lockfile.lock_exclusive()?;

        Ok(HostApp {
            root: PathBuf::from(root.as_ref()),
            lockfile,
            tempdir: TempDir::new()?,
            backups: Vec::new(),
        })
    }

    pub fn source_file(&mut self, name: &str) -> Result<File, Error> {
        let filepath = self.root.join("src").join(name);
        if filepath.exists() {
            let backup = self.tempdir.path().join(name);
            if backup.exists() {
                Err(format_err!(
                    "cannot overwrite source file '{}': already overwritten",
                    name
                ))?
            }
            self.backups.push((backup.clone(), filepath.clone()));
            fs::rename(&filepath, backup)?;
        }
        let f = File::create(filepath)?;
        Ok(f)
    }

    pub fn run<P: AsRef<Path>>(&self, guest_path: P) -> Result<(), Error> {
        let run_cargo = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("lucet-idl-test-rust-host")
            .arg("--")
            .arg(guest_path.as_ref())
            .current_dir(&self.root)
            .status()
            .expect("run cargo build");
        if !run_cargo.success() {
            Err(format_err!("cargo died building host project"))?
        }
        Ok(())
    }
}

impl Drop for HostApp {
    fn drop(&mut self) {
        for (backup, orig) in self.backups.iter() {
            fs::rename(backup, orig).expect("restore backup")
        }
        self.lockfile.unlock().expect("unlock");
    }
}
