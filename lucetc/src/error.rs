use crate::types::SignatureError;
use cranelift_module::ModuleError as ClifModuleError;
use cranelift_wasm::WasmError as ClifWasmError;
use faerie::ArtifactError;
use lucet_module::error::Error as LucetModuleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    //
    // General #[from] implementations.
    #[error("Builtins: {0}")]
    Builtins(#[from] parity_wasm::elements::Error),
    #[error("Clif module: {0}")]
    ClifModuleError(#[from] ClifModuleError),
    #[error("Translating: {0}")]
    ClifWasmError(#[from] ClifWasmError),
    #[error("Lucet Module: {0}")]
    LucetModule(#[from] LucetModuleError),
    #[error("Lucet validation: {0}")]
    LucetValidation(#[from] lucet_validate::Error),
    #[error("I/O: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Converting to Wasm signature: {0}")]
    SignatureConversion(#[from] SignatureError),
    #[error("Wasm validation: {0}")]
    WasmValidation(#[from] wasmparser::BinaryReaderError),
    #[error("Wat input: {0}")]
    WatInput(#[from] wabt::Error),
    //
    // Cannot apply #[from] or #[source] to these error types due to missing traits.
    #[error("Artifact: {1}. {0:?}")]
    ArtifactError(ArtifactError, String),
    #[error("Failure: {1}. {0:?}")]
    Failure(anyhow::Error, String),
    #[error("Patcher: {0:?}")]
    Patcher(wasmonkey::WError),
    //
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
        source: ClifWasmError,
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
    #[error("Trap records are present for function {0} but the function does not exist.")]
    TrapRecord(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("host machine is not a supported target: {0}")]
    UnsupportedIsa(#[from] cranelift_codegen::isa::LookupError),
}
