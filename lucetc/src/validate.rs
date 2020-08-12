mod moduletype;

use std::path::Path;
use std::rc::Rc;
use thiserror::Error;

use witx::{self, Id, Module};

pub use self::moduletype::{ImportFunc, ModuleType};
pub use wasmparser::FuncType;
pub use witx::{AtomType, Document, WitxError};

#[derive(Debug, Error)]
pub enum Error {
    #[error("WebAssembly validation error at offset {1}: {0}")]
    WasmValidation(String, usize),
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
        expected: FuncType,
        got: FuncType,
    },
    #[error("Export type error: for {field}, expected {expected:?}, got {got:?}")]
    ExportTypeError {
        field: String,
        expected: FuncType,
        got: FuncType,
    },
}

impl From<wasmparser::BinaryReaderError> for Error {
    fn from(e: wasmparser::BinaryReaderError) -> Error {
        Error::WasmValidation(e.message().to_owned(), e.offset())
    }
}

#[derive(Debug, Clone)]
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

    pub fn load<P: AsRef<Path>>(source_paths: &[P]) -> Result<Self, WitxError> {
        let witx = witx::load(source_paths)?;
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
            let spec_type = witx_to_functype(&func.core_type());
            if spec_type != import.ty {
                return Err(Error::ImportTypeError {
                    module: import.module,
                    field: import.field,
                    got: import.ty,
                    expected: spec_type,
                });
            }
        }

        if self.wasi_exe {
            self.check_wasi_start_func(&moduletype)?;
        }

        Ok(())
    }

    pub fn doc(&self) -> &Document {
        &self.witx
    }

    fn witx_module(&self, module: &str) -> Result<Rc<Module>, Error> {
        self.witx
            .module(&Id::new(module))
            .ok_or_else(|| Error::ModuleNotFound(module.to_string()))
    }

    fn check_wasi_start_func(&self, moduletype: &ModuleType) -> Result<(), Error> {
        let start_name = "_start";
        let expected = FuncType {
            params: vec![].into_boxed_slice(),
            returns: vec![].into_boxed_slice(),
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

fn witx_to_functype(coretype: &witx::CoreFuncType) -> FuncType {
    fn atom_to_type(atom: &witx::AtomType) -> wasmparser::Type {
        match atom {
            witx::AtomType::I32 => wasmparser::Type::I32,
            witx::AtomType::I64 => wasmparser::Type::I64,
            witx::AtomType::F32 => wasmparser::Type::F32,
            witx::AtomType::F64 => wasmparser::Type::F64,
        }
    }
    let params = coretype
        .args
        .iter()
        .map(|a| atom_to_type(&a.repr()))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let returns = if let Some(ref r) = coretype.ret {
        vec![atom_to_type(&r.repr())].into_boxed_slice()
    } else {
        vec![].into_boxed_slice()
    };
    FuncType { params, returns }
}
