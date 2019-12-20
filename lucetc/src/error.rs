use crate::types::SignatureError;
use cranelift_module::ModuleError as ClifModuleError;
use lucet_module::error::Error as LucetModuleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Binding error")]
    Binding(#[from] LucetModuleError),
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
    #[error("Output: {message}")]
    Output { message: String },
    //    #[error("Output file")]
    //    OutputFile(#[from] std::result::Error),
    #[error("Output function: error writing function {name}")]
    OutputFunction { name: String },
    #[error("Signature error: {message}")]
    Signature { message: String },
    #[error("Error converting cranelift signature to wasm signature")]
    SignatureConversion(#[from] SignatureError),
    #[error("Table")]
    Table,
    #[error("Temp file")]
    TempFile(#[from] std::io::Error),
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
