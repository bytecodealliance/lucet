use crate::error::LucetcErrorKind;
use crate::function_manifest::write_function_manifest;
use crate::name::Name;
use crate::stack_probe;
use crate::traps::write_trap_tables;
use cranelift_codegen::{ir, isa};
use cranelift_faerie::FaerieProduct;
use faerie::Artifact;
use failure::{format_err, Error, ResultExt};
use lucet_module_data::FunctionSpec;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct CraneliftFuncs {
    funcs: HashMap<Name, ir::Function>,
    isa: Box<dyn isa::TargetIsa>,
}

impl CraneliftFuncs {
    pub fn new(funcs: HashMap<Name, ir::Function>, isa: Box<isa::TargetIsa>) -> Self {
        Self { funcs, isa }
    }
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
    pub fn new(
        mut product: FaerieProduct,
        mut function_manifest: Vec<(String, FunctionSpec)>,
    ) -> Result<Self, Error> {
        stack_probe::declare_and_define(&mut product)?;

        let trap_manifest = &product
            .trap_manifest
            .expect("trap manifest will be present");

        // Now that we have trap information, we can fix up FunctionSpec entries to have
        // correct `trap_length` values
        let mut function_map: HashMap<String, u32> = HashMap::new();
        for (i, (name, _)) in function_manifest.iter().enumerate() {
            function_map.insert(name.clone(), i as u32);
        }

        for sink in trap_manifest.sinks.iter() {
            if let Some(idx) = function_map.get(&sink.name) {
                let (_, fn_spec) = &mut function_manifest
                    .get_mut(*idx as usize)
                    .expect("index is valid");

                std::mem::replace::<FunctionSpec>(
                    fn_spec,
                    FunctionSpec::new(0, fn_spec.code_len(), 0, sink.sites.len() as u64),
                );
            } else {
                Err(format_err!("Inconsistent state: trap records present for function {} but the function does not exist?", sink.name))
                    .context(LucetcErrorKind::TranslatingModule)?;
            }
        }

        write_trap_tables(trap_manifest, &mut product.artifact)?;
        write_function_manifest(function_manifest.as_slice(), &mut product.artifact)?;

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
