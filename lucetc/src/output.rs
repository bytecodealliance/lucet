use crate::error::Error;
use crate::name::Name;
use cranelift_codegen::{ir, isa};
use cranelift_faerie::FaerieProduct;
use faerie::Artifact;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

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
            write_function(&mut buffer, func, &Some(self.isa.as_ref()).into()).map_err(|e| {
                let message = format!("{:?}", n);
                Error::OutputFunction(e, message)
            })?
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
    pub fn new(product: FaerieProduct) -> Result<Self, Error> {
        let obj = Self {
            artifact: product.artifact,
        };

        Ok(obj)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(|| {
            let message = format!("Path must be filename {:?}", path.as_ref());
            Error::Input(message);
        });
        let file = File::create(path)?;
        self.artifact
            .write(file)
            .map_err(|source| Error::FaerieArtifact(source, "Write error".to_owned()))?;
        Ok(())
    }
}
