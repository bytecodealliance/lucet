use crate::workspace::Workspace;
use failure::Error;
use lucet_idl::{self, Backend, Config, Package};
use lucet_wasi;
use lucet_wasi_sdk::{CompileOpts, Link};
use lucetc::{Lucetc, LucetcOpts};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct CGuestApp {
    work: Workspace,
}

impl CGuestApp {
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            work: Workspace::new()?,
        })
    }

    fn generate_idl_h(&mut self, package: &Package) -> Result<(), Error> {
        lucet_idl::codegen(
            package,
            &Config {
                backend: Backend::CGuest,
            },
            Box::new(File::create(self.work.source_path("idl.h"))?),
        )?;
        Ok(())
    }

    fn generate_main_c(&mut self) -> Result<(), Error> {
        let mut main_file = File::create(self.work.source_path("main.c"))?;
        main_file.write_all(
            b"
#include <stdio.h>
#include \"idl.h\"

int main(int argc, char* argv[]) {
    printf(\"hello, world from c guest\");
}",
        )?;
        Ok(())
    }

    pub fn build(&mut self, package: &Package) -> Result<PathBuf, Error> {
        self.generate_idl_h(package)?;
        self.generate_main_c()?;

        Link::new(&[self.work.source_path("main.c")])
            .with_include(self.work.source_path(""))
            .link(&self.work.output_path("out.wasm"))?;

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
