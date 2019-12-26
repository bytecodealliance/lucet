use crate::module;
use crate::types::SignatureError;
use cranelift_module::ModuleError as ClifModuleError;
//use cranelift_wasm::environ::spec::WasmError as ClifWasmError;
use lucet_module::error::Error as LucetModuleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Binding error")]
    Binding(#[from] LucetModuleError),
    #[error("Build error")]
    Build(#[from] parity_wasm::elements::Error),
    #[error("Function definition error in {symbol}")]
    FunctionDefinition {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Function index out of bounds: {0}")]
    FunctionIndex(module::UniqueFuncIndex),
    #[error("Function translation error in {symbol}")]
    FunctionTranslation {
        symbol: String,
   //     #[source]
  //      source: ClifWasmError,
    },
    #[error("Inconsistent state when translating module: global {0} is declared as an import but has no entry in imported_globals")]
    GlobalDeclarationError(u32),
    #[error("global {0} is initialized by referencing another global value, but the referenced global is not an import")]
    GlobalInitError(u32),
    #[error("v128const type is not supported: {0}")]
    GlobalUnsupported(u32),
    #[error("Cannot initialize data beyond linear memory's initial size")]
    InitData,
    #[error("Input")]
    Input,
    #[error("Memory specs")]
    MemorySpecs,
    #[error("Metadata serializer; start index pointed to a non-function")]
    // specifically non-ModuleData; this will go away soon
    MetadataSerializer,
    #[error("Module data")]
    ModuleData,
    #[error("Output: {0}")]
    Output(String),
    //    #[error("Output file")]
    //    OutputFile(#[from] std::result::Error),
    #[error("Output function: error writing function {0}")]
    OutputFunction(String),
    #[error("Signature error: {0}")]
    Signature(String),
    #[error("Error converting cranelift signature to wasm signature")]
    SignatureConversion(#[from] SignatureError),
    #[error("Table")]
    Table,
    #[error("Table index is out of bounds: {0}")]
    TableIndexError(String),
    #[error("Temp file")]
    TempFile(#[from] std::io::Error),
    #[error("Translating module")]
    TranslatingModule,
    #[error("Trap records are present for function {0} but the function does not exist.")]
    TrapRecord(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("Validation")]
    Validation,
    #[error("Wat input")]
    WatInput(#[from] wabt::Error),
}
