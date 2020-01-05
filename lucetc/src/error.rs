// use crate::types::SignatureError;  // TLC impl something so I can use this...
use cranelift_module::ModuleError as ClifModuleError;
use lucet_module::error::Error as LucetModuleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Artifact error: {0}")]
    ArtifactError(String),
    #[error("Binding error")]
    Binding(#[from] LucetModuleError),
    #[error("Box conversion")]
    BoxConversion,
    #[error("Build error")]
    Build(#[from] parity_wasm::elements::Error),
    #[error("Function definition error in {symbol}")]
    FunctionDefinition {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Function index out of bounds: {0}")]
    FunctionIndexError(String),
    #[error("Function translation error in {symbol}")]
    FunctionTranslation { symbol: String },
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
    #[error("Input")]
    Input,
    #[error("Manifest error declaring {0}")]
    ManifestDeclaration(String),
    #[error("Manifest error defining {0}")]
    ManifestDefinition(String),
    #[error("Manifest error linking {0}")]
    ManifestLinking(String),
    #[error("Memory specs: {0}")]
    MemorySpecs(String),
    #[error("Metadata serializer; start index pointed to a non-function")]
    // specifically non-ModuleData; this will go away soon
    MetadataSerializer,
    #[error("Module data")]
    ModuleData,
    #[error("Output: {0}")]
    Output(String),
    #[error("Output function: error writing function {0}")]
    OutputFunction(String),
    #[error("Patcher error")]
    Patcher,
    #[error("Path error: {0}")]
    PathError(String),
    #[error("Signature error: {0}")]
    Signature(String),
    #[error("Error converting cranelift signature to wasm signature: {0}")]
    //    SignatureConversion(#[from] SignatureError), // TLC I wish I could do this...
    SignatureConversion(String),
    #[error("Table")]
    Table,
    #[error("Table index is out of bounds: {0}")]
    TableIndexError(String),
    #[error("Temp file")]
    TempFile(#[from] std::io::Error),
    #[error("Translating module")]
    TranslatingModule,
    #[error("Error defining {0} writing the function trap table into the object")]
    TrapDefinition(String),
    #[error("Trap records are present for function {0} but the function does not exist.")]
    TrapRecord(String),
    #[error("Trap table error declaring {0}")]
    TrapTable(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("Validation")]
    Validation,
    #[error("Wat input")]
    WatInput(#[from] wabt::Error),
}
