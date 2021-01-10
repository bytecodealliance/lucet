use crate::types::SignatureError;
use crate::validate::Error as ValidationError;
use cranelift_module::ModuleError as ClifModuleError;
use cranelift_wasm::WasmError as ClifWasmError;
use lucet_module::error::Error as LucetModuleError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // General #[from] implementations.
    #[error("Clif module: {0}")]
    ClifModuleError(#[from] ClifModuleError),
    #[error("Validating WebAssembly: at offset {offset}, {message}")]
    WebAssemblyValidation { message: String, offset: usize },
    #[error("Translation internal error: {0}")]
    ClifWasmInternalError(#[source] ClifWasmError),
    #[error("Lucet Module: {0}")]
    LucetModule(#[from] LucetModuleError),
    #[error("Lucet validation errors: {0:?}")]
    LucetValidation(Vec<ValidationError>),
    #[error("I/O: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Converting to Wasm signature: {0}")]
    SignatureConversion(#[from] SignatureError),
    #[error("Input does not have Wasm preamble")]
    MissingWasmPreamble,
    #[error("Wasm validation: {0}")]
    WasmValidation(#[from] wasmparser::BinaryReaderError),
    #[error("Wat input: {0}")]
    WatInput(#[from] wabt::Error),
    #[error("Object artifact: {1}. {0:?}")]
    ObjectArtifact(#[source] object::write::Error, String),
    // And all the rest
    #[error("Function definition error in {symbol}")]
    FunctionDefinition {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Function index out of bounds: {0}")]
    FunctionIndexError(String),
    #[error("Function translation error in {symbol}")]
    FunctionTranslation {
        symbol: String,
        #[source]
        source: Box<Error>,
    },
    #[error("Inconsistent state when translating module: global {0} is declared as an import but has no entry in imported_globals")]
    GlobalDeclarationError(u32),
    #[error("global out of bounds: {0}")]
    GlobalIndexError(String),
    #[error("global {0} is initialized by referencing another global value, but the referenced global is not an import")]
    GlobalInitError(u32),
    #[error("v128const type is not supported: {0}")]
    GlobalUnsupported(u32),
    #[error("Cannot initialize data beyond linear memory's initial size")]
    InitData,
    #[error("Input error: {0}")]
    Input(String),
    #[error("Ld error: {0}")]
    LdError(String),
    #[error("Memory specs: {0}")]
    MemorySpecs(String),
    #[error("Metadata serializer; start index points to a non-function: {0}")]
    MetadataSerializer(#[source] ClifModuleError),
    #[error("Output function: error writing function {1}")]
    OutputFunction(#[source] std::fmt::Error, String),
    #[error("Signature error: {0}")]
    Signature(String),
    #[error("Table index is out of bounds: {0}")]
    TableIndexError(String),
    #[error("Initializer {0:?} out of range for {1:?}")]
    ElementInitializerOutOfRange(crate::module::TableElems, cranelift_wasm::Table),
    #[error("Trap records are present for function {0} but the function does not exist.")]
    TrapRecord(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("host machine is not a supported target: {0}")]
    UnsupportedIsa(#[from] cranelift_codegen::isa::LookupError),
}

impl From<ClifWasmError> for Error {
    fn from(e: ClifWasmError) -> Error {
        match e {
            ClifWasmError::User(e) => Error::Input(e),
            ClifWasmError::InvalidWebAssembly { message, offset } => {
                Error::WebAssemblyValidation { message, offset }
            }
            ClifWasmError::Unsupported(e) => Error::Unsupported(e),
            ClifWasmError::ImplLimitExceeded { .. } => Error::ClifWasmInternalError(e),
        }
    }
}
