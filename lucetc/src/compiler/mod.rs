pub mod data;
pub mod entity;
pub mod function;
pub mod globals;
pub mod module_data;
pub mod opcode;
pub mod state;
pub mod table;
pub mod traps;

mod name;
mod stack_probe;

pub use self::name::Name;

use crate::compiler::traps::write_trap_manifest;
use crate::program::{Function, Program, TableDef};
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{ir, isa, print_errors::pretty_error, CodegenError};
use cranelift_faerie::{FaerieBackend, FaerieBuilder, FaerieProduct, FaerieTrapCollection};
use cranelift_module::{DataContext, Linkage, Module, ModuleError};
use cranelift_native;
use faerie::Artifact;
use failure::{format_err, Error, ResultExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum OptLevel {
    Default,
    Best,
    Fastest,
}

impl Default for OptLevel {
    fn default() -> OptLevel {
        OptLevel::Default
    }
}

impl OptLevel {
    fn to_flag(&self) -> &str {
        match self {
            OptLevel::Default => "default",
            OptLevel::Best => "best",
            OptLevel::Fastest => "fastest",
        }
    }
}

fn isa(opt_level: OptLevel) -> Box<isa::TargetIsa> {
    let mut flags_builder = settings::builder();
    let isa_builder = cranelift_native::builder().expect("host machine is not a supported target");
    flags_builder.enable("enable_verifier").unwrap();
    flags_builder.enable("is_pic").unwrap();
    flags_builder.set("opt_level", opt_level.to_flag()).unwrap();
    isa_builder.finish(settings::Flags::new(flags_builder))
}

pub struct Compiler<'p> {
    pub prog: &'p Program,
    funcs: HashMap<Name, ir::Function>,
    module: Module<FaerieBackend>,
    opt_level: OptLevel,
}

impl<'p> Compiler<'p> {
    pub fn new(name: String, prog: &'p Program, opt_level: OptLevel) -> Result<Self, Error> {
        let libcalls = Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
            _ => (FaerieBuilder::default_libcall_names())(libcall),
        });

        let mut compiler = Self {
            funcs: HashMap::new(),
            module: Module::new(FaerieBuilder::new(
                isa(opt_level),
                name,
                FaerieTrapCollection::Enabled,
                libcalls,
            )?),
            prog: prog,
            opt_level: opt_level,
        };

        for f in prog.import_functions() {
            compiler.declare_function(f)?;
        }

        let start_section = prog.module().start_section();
        for f in prog.defined_functions() {
            let name = compiler.declare_function(f)?;
            if Some(f.wasmidx) == start_section {
                compiler.define_start_symbol(&name)?;
            }
        }

        for f in prog.runtime_functions() {
            compiler.declare_function(f)?;
        }

        for t in prog.tables() {
            compiler.declare_table(t)?;
        }
        Ok(compiler)
    }

    pub fn isa(&self) -> Box<isa::TargetIsa> {
        isa(self.opt_level)
    }

    /// Add a `guest_start` data symbol pointing to the `start` section.
    ///
    /// We want to have the symbol `guest_start` point to the function
    /// designated in the `start` section of the wasm module, but we
    /// also want whatever function that is to be callable by its
    /// normal symbol. Since ELF doesn't support aliasing function
    /// symbols, we add a data symbol with a reloc pointer to the
    /// function's normal symbol.
    pub fn define_start_symbol(&mut self, start_func: &Name) -> Result<(), Error> {
        let name = self.declare_data("guest_start", Linkage::Export, false)?;
        let mut ctx = DataContext::new();
        ctx.define_zeroinit(8);
        let fid = start_func
            .into_funcid()
            .ok_or(format_err!("start index pointed to a non-function"))?;
        let fref = self.module.declare_func_in_data(fid, &mut ctx);
        ctx.write_function_addr(0, fref);
        self.define_data(name, &ctx)
    }

    pub fn declare_function(&mut self, func: &Function) -> Result<Name, Error> {
        let funcid = self
            .module
            .declare_function(&func.symbol(), func.linkage(), &func.signature())
            .context(format!("declaration of {}", func.symbol()))?;
        Ok(Name::new_func(func.symbol().to_owned(), funcid))
    }

    pub fn declare_table(&mut self, table: &TableDef) -> Result<Name, Error> {
        let mut serialized_len: Vec<u8> = Vec::new();
        serialized_len
            .write_u64::<LittleEndian>(table.len() as u64)
            .unwrap();
        let mut len_ctx = DataContext::new();
        len_ctx.define(serialized_len.into_boxed_slice());
        let len_decl = self
            .module
            .declare_data(&table.len_symbol(), Linkage::Export, false)?;
        self.module.define_data(len_decl, &len_ctx)?;

        let dataid = self
            .module
            .declare_data(&table.symbol(), Linkage::Export, false)?;
        Ok(Name::new_data(table.symbol(), dataid))
    }

    pub fn declare_data(
        &mut self,
        sym: &str,
        linkage: Linkage,
        mutable: bool,
    ) -> Result<Name, Error> {
        let dataid = self.module.declare_data(sym, linkage, mutable)?;
        Ok(Name::new_data(sym.to_owned(), dataid))
    }

    pub fn get_function(&self, func: &Function) -> Result<Name, Error> {
        let ident = self
            .module
            .get_name(&func.symbol())
            .ok_or(format_err!("function named {} undeclared", func.symbol()))?;
        Ok(Name::new(func.symbol().to_owned(), ident))
    }

    pub fn get_table(&self, table: &TableDef) -> Result<Name, Error> {
        let ident = self
            .module
            .get_name(&table.symbol())
            .ok_or(format_err!("table named {} undeclared", table.symbol()))?;
        Ok(Name::new(table.symbol(), ident))
    }

    pub fn get_data(&self, name: &str) -> Result<Name, Error> {
        let ident = self
            .module
            .get_name(name)
            .ok_or(format_err!("data named {} undeclared", name,))?;
        Ok(Name::new(name.to_owned(), ident))
    }

    pub fn define_function(&mut self, name: Name, func: ir::Function) -> Result<(), Error> {
        use std::collections::hash_map::Entry;
        match self.funcs.entry(name.clone()) {
            Entry::Occupied(_entry) => {
                return Err(format_err!(
                    "function {} has duplicate definition",
                    name.symbol()
                ));
            }
            Entry::Vacant(entry) => {
                entry.insert(func);
            }
        }
        Ok(())
    }

    pub fn define_data(&mut self, name: Name, data: &DataContext) -> Result<(), Error> {
        let id = name
            .into_dataid()
            .ok_or(format_err!("data defined with invalid name {:?}", name))?;
        self.module.define_data(id, data)?;
        Ok(())
    }

    pub fn cranelift_funcs(self) -> CraneliftFuncs {
        let isa = self.isa();
        CraneliftFuncs {
            funcs: self.funcs,
            isa: isa,
        }
    }

    pub fn codegen(self) -> Result<ObjectFile, Error> {
        use cranelift_codegen::Context;

        let isa = &*self.isa();
        let mut ctx = Context::new();
        let mut module = self.module;

        for (name, func) in self.funcs.iter() {
            ctx.func = func.clone();
            let id = name
                .into_funcid()
                .ok_or(format_err!("function defined with invalid name {:?}", name,))?;
            module.define_function(id, &mut ctx).map_err(|e| match e {
                ModuleError::Compilation(ce) => match ce {
                    CodegenError::Verifier(_) =>
                    // Verifier errors are never recoverable. This is the last
                    // time we'll have enough information still around to pretty-print
                    {
                        format_err!(
                            "code generation error:\n{}",
                            pretty_error(func, Some(isa), ce)
                        )
                    }
                    _ => ModuleError::Compilation(ce).into(),
                },
                _ => e.into(),
            })?;
            ctx.clear();
        }

        ObjectFile::new(module.finish())
    }
}

pub struct CraneliftFuncs {
    funcs: HashMap<Name, ir::Function>,
    isa: Box<isa::TargetIsa>,
}

impl CraneliftFuncs {
    /// This outputs a .clif file
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        use cranelift_codegen::write_function;
        let mut buffer = String::new();
        for (n, func) in self.funcs.iter() {
            buffer.push_str(&format!("; {}\n", n.symbol()));
            write_function(&mut buffer, func, Some(self.isa.as_ref()))
                .context(format_err!("writing func {:?}", n))?
        }
        let mut file = File::create(path)?;
        file.write_all(buffer.as_bytes())?;
        Ok(())
    }
}

pub struct ObjectFile {
    artifact: Artifact,
}
impl ObjectFile {
    pub fn new(mut product: FaerieProduct) -> Result<Self, Error> {
        stack_probe::declare_and_define(&mut product)?;
        let trap_manifest = &product
            .trap_manifest
            .expect("trap manifest will be present");
        write_trap_manifest(trap_manifest, &mut product.artifact)?;
        Ok(Self {
            artifact: product.artifact,
        })
    }
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(format_err!(
            "path {:?} needs to have filename",
            path.as_ref()
        ));
        let file = File::create(path)?;
        self.artifact.write(file)?;
        Ok(())
    }
}
