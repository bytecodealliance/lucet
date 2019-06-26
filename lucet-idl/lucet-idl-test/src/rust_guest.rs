use crate::workspace::Workspace;
use failure::{format_err, Error};
use lucet_idl::{self, Backend, Config, Package};
use lucet_wasi;
use lucetc::{Lucetc, LucetcOpts};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub struct RustGuestApp {
    work: Workspace,
}

impl RustGuestApp {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            work: Workspace::new()?,
        })
    }

    fn generate_idl_rs(&mut self, package: &Package) -> Result<(), Error> {
        lucet_idl::codegen(
            package,
            &Config {
                backend: Backend::RustGuest,
            },
            Box::new(File::create(self.work.source_path("idl.rs"))?),
        )?;
        Ok(())
    }

    fn generate_main_rs(&mut self) -> Result<(), Error> {
        let mut main_file = File::create(self.work.source_path("main.rs"))?;
        main_file.write_all(
            b"
#[allow(unused)]
mod idl;

fn main() {
    println!(\"hello, world from rust guest\");
}",
        )?;
        Ok(())
    }

    fn rustc(&mut self) -> Result<(), Error> {
        let cmd_rustc = Command::new("rustc")
            .arg("+nightly")
            .arg(self.work.source_path("main.rs"))
            .arg("--target=wasm32-wasi")
            .arg("-o")
            .arg(self.work.output_path("out.wasm"))
            .status()?;
        if !cmd_rustc.success() {
            Err(format_err!("rustc error building guest"))?
        }
        Ok(())
    }

    pub fn build(&mut self, package: &Package) -> Result<PathBuf, Error> {
        self.generate_idl_rs(package)?;
        self.generate_main_rs()?;
        self.rustc()?;
        let mut bindings = lucet_wasi::bindings();
        bindings.extend(&package.bindings())?;
        let lucetc = Lucetc::new(self.work.output_path("out.wasm")).with_bindings(bindings);
        let so_file = self.work.output_path("out.so");
        lucetc.shared_object_file(&so_file)?;
        Ok(so_file)
    }

    pub fn into_workspace(self) -> Workspace {
        self.work
    }
}
