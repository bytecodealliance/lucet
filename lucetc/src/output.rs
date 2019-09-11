use crate::name::Name;
use cranelift_codegen::{ir, isa};
use failure::{format_err, Error, ResultExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

mod faerie;
pub use crate::output::faerie::FaerieFile;
mod object;
pub use crate::output::object::ObjectFile;

pub const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

pub struct CraneliftFuncs {
    funcs: HashMap<Name, ir::Function>,
    isa: Box<dyn isa::TargetIsa>,
}

impl CraneliftFuncs {
    pub fn new(funcs: HashMap<Name, ir::Function>, isa: Box<dyn isa::TargetIsa>) -> Self {
        Self { funcs, isa }
    }
    /// This outputs a .clif file
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        use cranelift_codegen::write_function;
        let mut buffer = String::new();
        for (n, func) in self.funcs.iter() {
            buffer.push_str(&format!("; {}\n", n.symbol()));
            write_function(&mut buffer, func, &Some(self.isa.as_ref()).into())
                .context(format_err!("writing func {:?}", n))?
        }
        let mut file = File::create(path)?;
        file.write_all(buffer.as_bytes())?;
        Ok(())
    }
}
