mod moduletype;
mod types;
mod witx_moduletype;

use failure::Fail;
use std::rc::Rc;
use wasmparser;
use witx::{Document, Id, Module};

pub use self::moduletype::ModuleType;
pub use self::types::{AtomType, FuncSignature, ImportFunc};
pub use self::witx_moduletype::{HasFuncSignature, ModuleTypeParams, SignatureError};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "WebAssembly validation error at offset {}: {}", _1, 0)]
    WasmValidation(&'static str, usize),
    #[fail(display = "Unsupported: {}", _0)]
    Unsupported(String),
    #[fail(display = "{}", _0)]
    Signature(SignatureError),
    #[fail(display = "Uncategorized error: {}", _0)]
    Uncategorized(String),
}

impl From<SignatureError> for Error {
    fn from(e: SignatureError) -> Error {
        Error::Signature(e)
    }
}
impl From<wasmparser::BinaryReaderError> for Error {
    fn from(e: wasmparser::BinaryReaderError) -> Error {
        Error::WasmValidation(e.message, e.offset)
    }
}

pub fn validate(witx_doc: &Document, module_contents: &[u8], wasi_exe: bool) -> Result<(), Error> {
    wasmparser::validate(module_contents, None)?;

    let moduletype = ModuleType::parse_wasm(module_contents)?;

    for import in moduletype.imports() {
        let func = witx_module(witx_doc, &import.module)?
            .func(&Id::new(&import.field))
            .ok_or_else(|| {
                Error::Uncategorized(format!(
                    "func {}::{} not found",
                    import.module, import.field
                ))
            })?;
        let sig = func.func_signature()?;
        if sig != import.ty {
            Err(Error::Uncategorized(format!(
                "type mismatch in {}::{}: module has {:?}, spec has {:?}",
                import.module, import.field, import.ty, sig,
            )))?;
        }
    }

    if wasi_exe {
        check_wasi_start_func(&moduletype)?;
    }

    Ok(())
}

pub fn witx_module(doc: &Document, module: &str) -> Result<Rc<Module>, Error> {
    match module {
        "wasi_unstable" => doc.module(&Id::new("wasi_unstable_preview0")),
        _ => doc.module(&Id::new(module)),
    }
    .ok_or_else(|| Error::Uncategorized(format!("module {} not found", module)))
}

pub fn check_wasi_start_func(moduletype: &ModuleType) -> Result<(), Error> {
    if let Some(startfunc) = moduletype.export("_start") {
        if !(startfunc.params.is_empty() && startfunc.results.is_empty()) {
            Err(Error::Uncategorized(format!(
                "bad type signature on _start: {:?}",
                startfunc
            )))
        } else {
            Ok(())
        }
    } else {
        Err(Error::Uncategorized(
            "missing WASI executable start function (\"_start\")".to_string(),
        ))
    }
}
