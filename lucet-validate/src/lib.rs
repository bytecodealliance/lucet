mod moduletype;

use failure::Fail;
use wasmparser;
use witx;

pub use self::moduletype::{FuncSignature, ImportFunc, ModuleType};

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "WebAssembly validation error at offset {}: {}", _1, 0)]
    WasmValidation(&'static str, usize),
    #[fail(display = "Unsupported: {}", _0)]
    Unsupported(String),
}

impl From<wasmparser::BinaryReaderError> for Error {
    fn from(e: wasmparser::BinaryReaderError) -> Error {
        Error::WasmValidation(e.message, e.offset)
    }
}

pub fn validate(interface: &witx::Document, module_contents: &[u8]) -> Result<(), Error> {
    wasmparser::validate(module_contents, None)?;

    let moduletype = ModuleType::parse(module_contents)?;

    for import in moduletype.imports() {
        println!(
            "import {}::{} has type {:?}",
            import.module, import.field, import.ty
        );
    }

    if let Some(startfunc) = moduletype.export("_start") {
        println!("wasi start func has type {:?}", startfunc);
    } else {
        println!("no wasi start func");
    }

    Ok(())
}
