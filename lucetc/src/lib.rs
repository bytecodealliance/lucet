pub mod bindings;
pub mod compiler;
pub mod error;
pub mod load;
pub mod patch;
pub mod program;

use crate::compiler::function::compile_function;
use crate::compiler::module_data::compile_module_data;
use crate::compiler::table::compile_table;
use crate::error::{LucetcError, LucetcErrorKind};
use crate::load::read_module;
use crate::patch::patch_module;
use crate::program::Program;
use failure::{format_err, Error, ResultExt};
use parity_wasm::elements::Module;
use std::env;
use std::path::{Path, PathBuf};
use tempfile;

pub use crate::{
    bindings::Bindings,
    compiler::{Compiler, OptLevel},
    program::memory::HeapSettings,
};

pub struct Lucetc {
    input: PathBuf,
    bindings: Vec<Bindings>,
    opt_level: OptLevel,
    heap: HeapSettings,
    builtins_paths: Vec<PathBuf>,
}

pub trait AsLucetc {
    fn as_lucetc(&mut self) -> &mut Lucetc;
}

impl AsLucetc for Lucetc {
    fn as_lucetc(&mut self) -> &mut Lucetc {
        self
    }
}

pub trait LucetcOpts {
    fn bindings(&mut self, bindings: Bindings);
    fn with_bindings(self, bindings: Bindings) -> Self;

    fn opt_level(&mut self, opt_level: OptLevel);
    fn with_opt_level(self, opt_level: OptLevel) -> Self;

    fn builtins<P: AsRef<Path>>(&mut self, builtins_path: P);
    fn with_builtins<P: AsRef<Path>>(self, builtins_path: P) -> Self;

    fn min_reserved_size(&mut self, min_reserved_size: u64);
    fn with_min_reserved_size(self, min_reserved_size: u64) -> Self;

    fn max_reserved_size(&mut self, max_reserved_size: u64);
    fn with_max_reserved_size(self, max_reserved_size: u64) -> Self;

    fn guard_size(&mut self, guard_size: u64);
    fn with_guard_size(self, guard_size: u64) -> Self;
}

impl<T: AsLucetc> LucetcOpts for T {
    fn bindings(&mut self, bindings: Bindings) {
        self.as_lucetc().bindings.push(bindings);
    }

    fn with_bindings(mut self, bindings: Bindings) -> Self {
        self.bindings(bindings);
        self
    }

    fn opt_level(&mut self, opt_level: OptLevel) {
        self.as_lucetc().opt_level = opt_level;
    }

    fn with_opt_level(mut self, opt_level: OptLevel) -> Self {
        self.opt_level(opt_level);
        self
    }

    fn builtins<P: AsRef<Path>>(&mut self, builtins_path: P) {
        self.as_lucetc()
            .builtins_paths
            .push(builtins_path.as_ref().to_owned());
    }

    fn with_builtins<P: AsRef<Path>>(mut self, builtins_path: P) -> Self {
        self.builtins(builtins_path);
        self
    }

    fn min_reserved_size(&mut self, min_reserved_size: u64) {
        self.as_lucetc().heap.min_reserved_size = min_reserved_size;
    }

    fn with_min_reserved_size(mut self, min_reserved_size: u64) -> Self {
        self.min_reserved_size(min_reserved_size);
        self
    }

    fn max_reserved_size(&mut self, max_reserved_size: u64) {
        self.as_lucetc().heap.max_reserved_size = max_reserved_size;
    }

    fn with_max_reserved_size(mut self, max_reserved_size: u64) -> Self {
        self.max_reserved_size(max_reserved_size);
        self
    }

    fn guard_size(&mut self, guard_size: u64) {
        self.as_lucetc().heap.guard_size = guard_size;
    }

    fn with_guard_size(mut self, guard_size: u64) -> Self {
        self.guard_size(guard_size);
        self
    }
}

impl Lucetc {
    pub fn new<P: AsRef<Path>>(input: P) -> Self {
        let input = input.as_ref();
        Self {
            input: input.to_owned(),
            bindings: vec![],
            opt_level: OptLevel::default(),
            heap: HeapSettings::default(),
            builtins_paths: vec![],
        }
    }

    fn build(&self) -> Result<(String, Module, Bindings), Error> {
        let name = String::from(
            self.input
                .file_stem()
                .ok_or(format_err!("input filename {:?} is empty", self.input))?
                .to_str()
                .ok_or(format_err!(
                    "input filename {:?} is not valid utf8",
                    self.input
                ))?,
        );
        let mut builtins_bindings = vec![];
        let mut module = read_module(&self.input)?;

        for builtins in self.builtins_paths.iter() {
            let (newmodule, builtins_map) = patch_module(module, builtins)?;
            module = newmodule;
            builtins_bindings.push(Bindings::env(builtins_map));
        }

        let mut bindings = Bindings::empty();

        for binding in builtins_bindings.iter().chain(self.bindings.iter()) {
            bindings.extend(binding)?;
        }

        Ok((name, module, bindings))
    }

    pub fn object_file<P: AsRef<Path>>(self, output: P) -> Result<(), Error> {
        let (name, module, bindings) = self.build()?;

        let prog = Program::new(module, bindings, self.heap)?;
        let comp = compile(&prog, &name, self.opt_level)?;

        let obj = comp.codegen()?;
        obj.write(output.as_ref()).context("writing object file")?;

        Ok(())
    }

    pub fn clif_ir<P: AsRef<Path>>(self, output: P) -> Result<(), Error> {
        let (name, module, bindings) = self.build()?;

        let prog = Program::new(module, bindings, self.heap.clone())?;
        let comp = compile(&prog, &name, self.opt_level)?;

        comp.cranelift_funcs()
            .write(&output)
            .context("writing clif file")?;

        Ok(())
    }

    pub fn shared_object_file<P: AsRef<Path>>(self, output: P) -> Result<(), Error> {
        let dir = tempfile::Builder::new().prefix("lucetc").tempdir()?;
        let objpath = dir.path().join("tmp.o");
        self.object_file(objpath.clone())?;
        link_so(objpath, output)?;
        Ok(())
    }
}

const LD_DEFAULT: &str = "ld";

#[cfg(not(target_os = "macos"))]
const LDFLAGS_DEFAULT: &str = "-shared";

#[cfg(target_os = "macos")]
const LDFLAGS_DEFAULT: &str = "-dylib -dead_strip -export_dynamic -undefined dynamic_lookup";

fn link_so<P, Q>(objpath: P, sopath: Q) -> Result<(), Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    use std::process::Command;
    let mut cmd_ld = Command::new(env::var("LD").unwrap_or(LD_DEFAULT.into()));
    cmd_ld.arg(objpath.as_ref());
    let env_ldflags = env::var("LDFLAGS").unwrap_or(LDFLAGS_DEFAULT.into());
    for flag in env_ldflags.split_whitespace() {
        cmd_ld.arg(flag);
    }
    cmd_ld.arg("-o");
    cmd_ld.arg(sopath.as_ref());

    let run_ld = cmd_ld
        .output()
        .context(format_err!("running ld on {:?}", objpath.as_ref()))?;

    if !run_ld.status.success() {
        Err(format_err!(
            "ld of {} failed: {}",
            objpath.as_ref().to_str().unwrap(),
            String::from_utf8_lossy(&run_ld.stderr)
        ))?;
    }
    Ok(())
}

pub fn compile<'p>(
    program: &'p Program,
    name: &str,
    opt_level: OptLevel,
) -> Result<Compiler<'p>, LucetcError> {
    let mut compiler = Compiler::new(name.to_owned(), &program, opt_level)?;

    compile_module_data(&mut compiler).context(LucetcErrorKind::ModuleData)?;

    for function in program.defined_functions() {
        let body = program.function_body(&function);
        compile_function(&mut compiler, &function, body)
            .context(LucetcErrorKind::Function(function.symbol().to_owned()))?;
    }
    for table in program.tables() {
        compile_table(&mut compiler, &table)
            .context(LucetcErrorKind::Table(table.symbol().to_owned()))?;
    }

    Ok(compiler)
}
