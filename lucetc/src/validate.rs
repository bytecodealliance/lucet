pub mod moduletype;

use std::path::Path;
use std::rc::Rc;
use thiserror::Error;

use witx::{self, Id, Module};

pub use self::moduletype::{ImportFunc, ModuleType};
pub use wasmparser::FuncType;
pub use witx::{AtomType, Document, WitxError};

#[derive(Debug, Error)]
pub enum Error {
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

#[derive(Debug, Clone)]
pub struct Validator {
    witx: Document,
    required_exports: Vec<(&'static str, FuncType)>,
}

impl Validator {
    pub fn new(witx: Document, wasi_exe: bool) -> Self {
        Self {
            witx,
            required_exports: vec![],
        }
        .with_wasi_exe(wasi_exe)
    }

    pub fn parse(source: &str) -> Result<Self, WitxError> {
        let witx = witx::parse(source)?;
        Ok(Self::new(witx, false))
    }

    pub fn load<P: AsRef<Path>>(source_paths: &[P]) -> Result<Self, WitxError> {
        let witx = witx::load(source_paths)?;
        Ok(Self::new(witx, false))
    }

    pub fn wasi_exe(&mut self, check: bool) {
        if check {
            self.required_exports = vec![(
                "_start",
                FuncType {
                    params: vec![].into_boxed_slice(),
                    returns: vec![].into_boxed_slice(),
                },
            )];
        } else {
            self.required_exports = vec![];
        }
    }

    pub fn with_wasi_exe(mut self, check: bool) -> Self {
        self.wasi_exe(check);
        self
    }

    pub fn available_import(
        &self,
        module: &str,
        field: &str,
        type_: &FuncType,
    ) -> Result<(), Error> {
        let func = self
            .witx_module(module)?
            .func(&Id::new(field))
            .ok_or_else(|| Error::ImportNotFound {
                module: module.to_owned(),
                field: field.to_owned(),
            })?;
        let spec_type = witx_to_functype(&func.core_type());
        if &spec_type != type_ {
            return Err(Error::ImportTypeError {
                module: module.to_owned(),
                field: field.to_owned(),
                got: type_.clone(),
                expected: spec_type,
            });
        } else {
            Ok(())
        }
    }

    pub fn required_exports(&self) -> &[(&'static str, FuncType)] {
        &self.required_exports
    }

    pub fn validate_module_type(&self, moduletype: &ModuleType) -> Result<(), Error> {
        /*
        wasmparser::validate(module_contents, None)?;

        let moduletype = ModuleType::parse_wasm(module_contents)?;
        */

        for import in moduletype.imports() {
            self.available_import(&import.module, &import.field, &import.ty)?;
        }

        for (name, expected) in self.required_exports.iter() {
            if let Some(e) = moduletype.export(name) {
                if e != expected {
                    return Err(Error::ExportTypeError {
                        field: name.to_string(),
                        expected: expected.clone(),
                        got: e.clone(),
                    });
                }
            } else {
                return Err(Error::ExportNotFound {
                    field: name.to_string(),
                });
            }
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
