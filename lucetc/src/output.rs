use crate::error::Error;
use crate::name::Name;
use cranelift_codegen::ir;
use cranelift_object::ObjectProduct;
use object::write::Object;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) const FUNCTION_MANIFEST_SYM: &str = "lucet_function_manifest";

pub struct CraneliftFuncs {
    funcs: HashMap<Name, ir::Function>,
}

impl CraneliftFuncs {
    pub fn new(funcs: HashMap<Name, ir::Function>) -> Self {
        Self { funcs }
    }
    /// This outputs a .clif file
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        use cranelift_codegen::write_function;
        let mut buffer = String::new();
        for (n, func) in self.funcs.iter() {
            buffer.push_str(&format!("; {}\n", n.symbol()));
            write_function(&mut buffer, func).map_err(|e| {
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
    object: Object<'static>,
}
impl ObjectFile {
    pub fn new(product: ObjectProduct) -> Result<Self, Error> {
        let obj = Self {
            object: product.object,
        };

        Ok(obj)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        let _ = path.as_ref().file_name().ok_or(|| {
            let message = format!("Path must be filename {:?}", path.as_ref());
            Error::Input(message);
        });
        let mut file = File::create(path)?;
        let bytes = self
            .object
            .write()
            .map_err(|source| Error::ObjectArtifact(source, "Write error".to_owned()))?;
        file.write_all(&bytes)?;
        Ok(())
    }
}
