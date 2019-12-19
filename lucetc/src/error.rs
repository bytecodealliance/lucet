use crate::types::SignatureError;
use cranelift_module::ModuleError as ClifModuleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Build error")]
    Build(#[from] parity_wasm::elements::Error),
    #[error("Function translation error in {symbol}")]
    FunctionTranslation {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Function definition error in {symbol}")]
    FunctionDefinition {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Inconsistent state when translating module: global {0} is declared as an import but has no entry in imported_globals")]
    GlobalDeclarationError(u32),
    #[error("Input")]
    Input,
    #[error("Memory specs")]
    MemorySpecs,
    #[error("Metadata serializer; start index pointed to a non-function")]
    // specifically non-ModuleData; this will go away soon
    MetadataSerializer,
    #[error("Module data")]
    ModuleData,
    #[error("Output")]
    Output,
    #[error("Signature error: {message}")]
    Signature { message: String },
    #[error("Error converting cranelift signature to wasm signature")]
    SignatureConversion(#[from] SignatureError),
    #[error("Table")]
    Table,
    #[error("Translating module")]
    TranslatingModule,
    #[error("Trap records are present for function {name} but the function does not exist.")]
    TrapRecord { name: String },
    #[error("Unsupported: {message}")]
    Unsupported { message: String },
    #[error("Validation: {message}")]
    Validation { message: String },
    #[error("Wat input")]
    WatInput(#[from] wabt::Error),
}

