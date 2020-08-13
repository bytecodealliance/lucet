pub mod moduletype;

use std::path::Path;
use std::rc::Rc;
use thiserror::Error;

use witx::{self, Id, Module};

use self::moduletype::ModuleType;

pub use wasmparser::FuncType;
pub use witx::{AtomType, Document, WitxError};

#[derive(Debug, Error, Clone)]
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

pub enum WasiMode {
    Command,
    Reactor,
}

pub struct ValidatorBuilder {
    witx: Vec<Document>,
    wasi_mode: Option<WasiMode>,
}

impl ValidatorBuilder {
    pub fn new() -> Self {
        Self {
            witx: vec![],
            wasi_mode: None,
        }
    }

    pub fn with_witx(&mut self, doc: Document) {
        self.witx.push(doc);
    }
    pub fn witx(mut self, doc: Document) -> Self {
        self.with_witx(doc);
        self
    }

    pub fn with_parse_witx(&mut self, source: &str) -> Result<(), WitxError> {
        let doc = witx::parse(source)?;
        self.with_witx(doc);
        Ok(())
    }
    pub fn parse_witx(mut self, source: &str) -> Result<Self, WitxError> {
        self.with_parse_witx(source)?;
        Ok(self)
    }

    pub fn with_load_witx(&mut self, source_paths: &[impl AsRef<Path>]) -> Result<(), WitxError> {
        let doc = witx::load(source_paths)?;
        self.with_witx(doc);
        Ok(())
    }
    pub fn load_witx(mut self, source_paths: &[impl AsRef<Path>]) -> Result<Self, WitxError> {
        self.with_load_witx(source_paths)?;
        Ok(self)
    }

    pub fn with_wasi_mode(&mut self, mode: Option<WasiMode>) {
        self.wasi_mode = mode;
    }
    pub fn wasi_mode(mut self, mode: Option<WasiMode>) -> Self {
        self.with_wasi_mode(mode);
        self
    }

    pub fn build(self) -> Validator {
        let no_params_no_returns = FuncType {
            params: vec![].into_boxed_slice(),
            returns: vec![].into_boxed_slice(),
        };
        let (required_exports, optional_exports) = match self.wasi_mode {
            None => (vec![], vec![]),
            Some(WasiMode::Command) => (vec![("_start", no_params_no_returns)], vec![]),
            Some(WasiMode::Reactor) => (vec![], vec![("_initialize", no_params_no_returns)]),
        };
        Validator {
            witx: self.witx,
            required_exports,
            optional_exports,
            errors: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validator {
    witx: Vec<Document>,
    required_exports: Vec<(&'static str, FuncType)>,
    optional_exports: Vec<(&'static str, FuncType)>,
    errors: Vec<Error>,
}

impl Validator {
    pub fn builder() -> ValidatorBuilder {
        ValidatorBuilder::new()
    }

    pub fn docs(&self) -> &[Document] {
        &self.witx
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

    fn witx_module(&self, module: &str) -> Result<Rc<Module>, Error> {
        self.witx
            .iter()
            .find_map(|doc| doc.module(&Id::new(module)))
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
