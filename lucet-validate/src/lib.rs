mod moduletype;
mod types;

use std::path::Path;
use std::rc::Rc;
use thiserror::Error;
use wasmparser;
use witx::{self, Id, Module};

pub use self::moduletype::ModuleType;
pub use self::types::{FuncSignature, ImportFunc};
pub use witx::{AtomType, Document, WitxError};

#[derive(Debug, Error)]
pub enum Error {
    #[error("WebAssembly validation error at offset {1}: {0}")]
    WasmValidation(&'static str, usize),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Import not found: {module}::{field}")]
    ImportNotFound { module: String, field: String },
    #[error("Export not found: {field}")]
    ExportNotFound { field: String },
    #[error("Import type error: for {module}::{field}, expected {expected:?}, got {got:?}")]
    ImportTypeError {
        module: String,
        field: String,
        expected: FuncSignature,
        got: FuncSignature,
    },
    #[error("Export type error: for {field}, expected {expected:?}, got {got:?}")]
    ExportTypeError {
        field: String,
        expected: FuncSignature,
        got: FuncSignature,
    },
}

impl From<wasmparser::BinaryReaderError> for Error {
    fn from(e: wasmparser::BinaryReaderError) -> Error {
        Error::WasmValidation(e.message, e.offset)
    }
}

pub struct Validator {
    witx: Document,
    wasi_exe: bool,
}

impl Validator {
    pub fn new(witx: Document, wasi_exe: bool) -> Self {
        Self { witx, wasi_exe }
    }

    pub fn parse(source: &str) -> Result<Self, WitxError> {
        let witx = witx::parse(source)?;
        Ok(Self {
            witx,
            wasi_exe: false,
        })
    }

    pub fn load<P: AsRef<Path>>(source_path: P) -> Result<Self, WitxError> {
        let witx = witx::load(&[source_path.as_ref()])?;
        Ok(Self {
            witx,
            wasi_exe: false,
        })
    }

    pub fn wasi_exe(&mut self, check: bool) {
        self.wasi_exe = check;
    }

    pub fn with_wasi_exe(mut self, check: bool) -> Self {
        self.wasi_exe(check);
        self
    }

    pub fn validate(&self, module_contents: &[u8]) -> Result<(), Error> {
        wasmparser::validate(module_contents, None)?;

        let moduletype = ModuleType::parse_wasm(module_contents)?;

        for import in moduletype.imports() {
            let func = self
                .witx_module(&import.module)?
                .func(&Id::new(&import.field))
                .ok_or_else(|| Error::ImportNotFound {
                    module: import.module.clone(),
                    field: import.field.clone(),
                })?;
            let spec_type = FuncSignature::from(func.core_type());
            if spec_type != import.ty {
                Err(Error::ImportTypeError {
                    module: import.module,
                    field: import.field,
                    got: import.ty,
                    expected: spec_type,
                })?;
            }
        }

        if self.wasi_exe {
            self.check_wasi_start_func(&moduletype)?;
        }

        Ok(())
    }

    fn witx_module(&self, module: &str) -> Result<Rc<Module>, Error> {
        self.witx
            .module(&Id::new(module))
            .ok_or_else(|| Error::ModuleNotFound(module.to_string()))
    }

    fn check_wasi_start_func(&self, moduletype: &ModuleType) -> Result<(), Error> {
        let start_name = "_start";
        let expected = FuncSignature {
            args: vec![],
            ret: None,
        };
        if let Some(startfunc) = moduletype.export(start_name) {
            if startfunc != &expected {
                Err(Error::ExportTypeError {
                    field: start_name.to_string(),
                    expected,
                    got: startfunc.clone(),
                })
            } else {
                Ok(())
            }
        } else {
            Err(Error::ExportNotFound {
                field: start_name.to_string(),
            })
        }
    }
}
