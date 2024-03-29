use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use witx::Id;

pub use cranelift_wasm::{WasmFuncType, WasmType};
pub use witx::{Document, WitxError};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Import not found: {module}::{field}")]
    ImportNotFound { module: String, field: String },
    #[error("Export not found: {field}")]
    ExportNotFound { field: String },
    #[error("Import type error: for {module}::{field}, expected {expected:?}, got {got:?}")]
    ImportTypeError {
        module: String,
        field: String,
        expected: WasmFuncType,
        got: WasmFuncType,
    },
    #[error("Export type error: for {field}, expected {expected:?}, got {got:?}")]
    ExportTypeError {
        field: String,
        expected: WasmFuncType,
        got: WasmFuncType,
    },
    #[error("Missing required export function: {field} with type {type_:?}")]
    MissingRequiredExport { field: String, type_: WasmFuncType },
}

#[derive(Debug, Clone)]
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
        let no_params_no_returns =
            WasmFuncType::new(vec![].into_boxed_slice(), vec![].into_boxed_slice());
        let exports = match self.wasi_mode {
            None => vec![],
            Some(WasiMode::Command) => vec![(
                "_start".to_string(),
                ExportRequired::Required,
                no_params_no_returns,
            )],
            Some(WasiMode::Reactor) => vec![(
                "_initialize".to_string(),
                ExportRequired::Optional,
                no_params_no_returns,
            )],
        };
        Validator::new(self.witx, exports.into_iter())
    }
}

#[derive(Debug, Clone)]
enum ExportRequired {
    Required,
    Optional,
}

#[derive(Debug, Clone)]
struct ExportSpec {
    name: String,
    required: ExportRequired,
    type_: WasmFuncType,
    result: Option<Result<(), Error>>,
}

impl ExportSpec {
    fn result(&self) -> Option<Error> {
        match (&self.result, &self.required) {
            (Some(Err(e)), _) => Some(e.clone()),
            (None, ExportRequired::Required) => Some(Error::MissingRequiredExport {
                field: self.name.clone(),
                type_: self.type_.clone(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validator {
    witx: Vec<Document>,
    exports: HashMap<String, ExportSpec>,
    import_errors: Vec<Error>,
}

impl Validator {
    pub fn builder() -> ValidatorBuilder {
        ValidatorBuilder::new()
    }

    fn new(
        witx: Vec<Document>,
        exports: impl Iterator<Item = (String, ExportRequired, WasmFuncType)>,
    ) -> Self {
        let exports = exports
            .map(|(name, required, type_)| {
                (
                    name.clone(),
                    ExportSpec {
                        name,
                        required,
                        type_,
                        result: None,
                    },
                )
            })
            .collect();
        Self {
            witx,
            exports,
            import_errors: vec![],
        }
    }

    /// Used to calculate bindings
    pub fn docs(&self) -> &[Document] {
        &self.witx
    }

    pub fn register_import(&mut self, module: &str, field: &str, type_: &WasmFuncType) {
        let inner = || {
            let not_found = Error::ImportNotFound {
                module: module.to_owned(),
                field: field.to_owned(),
            };
            let func = self
                .witx
                .iter()
                .find_map(|doc| doc.module(&Id::new(module)))
                .ok_or(not_found.clone())?
                .func(&Id::new(field))
                .ok_or(not_found)?;
            let spec_type = witx_to_functype(&func);
            if &spec_type != type_ {
                Err(Error::ImportTypeError {
                    module: module.to_owned(),
                    field: field.to_owned(),
                    got: type_.clone(),
                    expected: spec_type,
                })
            } else {
                Ok(())
            }
        };
        match inner() {
            Ok(()) => {}
            Err(e) => self.import_errors.push(e),
        }
    }

    pub fn register_export(&mut self, name: &str, type_: &WasmFuncType) {
        if let Some(mut e) = self.exports.get_mut(name) {
            if &e.type_ != type_ {
                e.result = Some(Err(Error::ExportTypeError {
                    field: name.to_owned(),
                    expected: e.type_.clone(),
                    got: type_.clone(),
                }))
            } else {
                e.result = Some(Ok(()))
            }
        }
    }

    pub fn report(&self) -> Result<(), Vec<Error>> {
        let mut errs = self.import_errors.clone();
        for (_n, ex) in self.exports.iter() {
            if let Some(err) = ex.result() {
                errs.push(err.clone());
            }
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }
}

fn witx_to_functype(func: &witx::InterfaceFunc) -> WasmFuncType {
    fn atom_to_type(atom: &witx::WasmType) -> WasmType {
        match atom {
            witx::WasmType::I32 => WasmType::I32,
            witx::WasmType::I64 => WasmType::I64,
            witx::WasmType::F32 => WasmType::F32,
            witx::WasmType::F64 => WasmType::F64,
        }
    }
    let (params, results) = func.wasm_signature();
    let params = params
        .iter()
        .map(|a| atom_to_type(&a))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let returns = match results.len() {
        0 => vec![].into_boxed_slice(),
        1 => vec![atom_to_type(&results[0])].into_boxed_slice(),
        _ => unimplemented!("multiple result types"),
    };
    WasmFuncType::new(params, returns)
}
