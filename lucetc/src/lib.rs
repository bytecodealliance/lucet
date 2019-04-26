mod bindings;
mod compiler;
mod decls;
mod error;
mod function;
mod function_manifest;
mod heap;
mod load;
mod module;
mod name;
mod output;
mod patch;
mod pointer;
mod runtime;
mod sparsedata;
mod stack_probe;
mod table;
mod traps;

pub use crate::{
    bindings::Bindings,
    compiler::Compiler,
    compiler::OptLevel,
    error::{LucetcError, LucetcErrorKind},
    heap::HeapSettings,
    load::read_module,
    patch::patch_module,
};
use failure::{format_err, Error, ResultExt};
use std::env;
use std::path::{Path, PathBuf};
use tempfile;

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

    /// Set the reserved size exactly.
    ///
    /// Equivalent to setting the minimum and maximum reserved sizes to the same value.
    fn reserved_size(&mut self, reserved_size: u64);
    /// Set the reserved size exactly.
    ///
    /// Equivalent to setting the minimum and maximum reserved sizes to the same value.
    fn with_reserved_size(self, reserved_size: u64) -> Self;

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

    fn reserved_size(&mut self, reserved_size: u64) {
        self.as_lucetc().heap.min_reserved_size = reserved_size;
        self.as_lucetc().heap.max_reserved_size = reserved_size;
    }

    fn with_reserved_size(mut self, reserved_size: u64) -> Self {
        self.reserved_size(reserved_size);
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

    fn build(&self) -> Result<(Vec<u8>, Bindings), Error> {
        use parity_wasm::elements::{deserialize_buffer, serialize};

        let mut builtins_bindings = vec![];
        let mut module_binary = read_module(&self.input)?;

        if !self.builtins_paths.is_empty() {
            let mut module = deserialize_buffer(&module_binary)?;
            for builtins in self.builtins_paths.iter() {
                let (newmodule, builtins_map) = patch_module(module, builtins)?;
                module = newmodule;
                builtins_bindings.push(Bindings::env(builtins_map));
            }
            module_binary = serialize(module)?;
        }

        let mut bindings = Bindings::empty();

        for binding in builtins_bindings.iter().chain(self.bindings.iter()) {
            bindings.extend(binding)?;
        }

        Ok((module_binary, bindings))
    }

    pub fn object_file<P: AsRef<Path>>(&self, output: P) -> Result<(), Error> {
        let (module_contents, bindings) = self.build()?;

        let compiler = Compiler::new(
            &module_contents,
            self.opt_level,
            &bindings,
            self.heap.clone(),
        )?;
        let obj = compiler.object_file()?;

        obj.write(output.as_ref()).context("writing object file")?;
        Ok(())
    }

    pub fn clif_ir<P: AsRef<Path>>(&self, output: P) -> Result<(), Error> {
        let (module_contents, bindings) = self.build()?;

        let compiler = Compiler::new(
            &module_contents,
            self.opt_level,
            &bindings,
            self.heap.clone(),
        )?;

        compiler
            .cranelift_funcs()?
            .write(&output)
            .context("writing clif file")?;

        Ok(())
    }

    pub fn shared_object_file<P: AsRef<Path>>(&self, output: P) -> Result<(), Error> {
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
