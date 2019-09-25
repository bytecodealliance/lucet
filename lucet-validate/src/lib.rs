mod moduletype;
mod types;
mod witx_moduletype;

use failure::Fail;
use wasmparser;
use witx;

pub use self::moduletype::ModuleType;
pub use self::types::{AtomType, FuncSignature, ImportFunc};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "WebAssembly validation error at offset {}: {}", _1, 0)]
    WasmValidation(&'static str, usize),
    #[fail(display = "Unsupported: {}", _0)]
    Unsupported(String),
    #[fail(display = "Uncategorized error: {}", _0)]
    Uncategorized(String),
}

impl From<wasmparser::BinaryReaderError> for Error {
    fn from(e: wasmparser::BinaryReaderError) -> Error {
        Error::WasmValidation(e.message, e.offset)
    }
}

pub fn validate(
    witx_doc: &witx::Document,
    module_contents: &[u8],
    wasi_exe: bool,
) -> Result<(), Error> {
    wasmparser::validate(module_contents, None)?;

    let moduletype = ModuleType::parse_wasm(module_contents)?;

    for import in moduletype.imports() {
        println!(
            "import {}::{} has type {:?}",
            import.module, import.field, import.ty
        );
    }

    if wasi_exe {
        check_wasi_start_func(&moduletype)?;
    }

    Ok(())
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
