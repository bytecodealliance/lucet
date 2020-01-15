// use crate::types::SignatureError;  // TLC impl something so I can use this...
use cranelift_module::ModuleError as ClifModuleError;
use cranelift_wasm::WasmError as ClifWasmError;
use faerie::ArtifactError;
use lucet_module::error::Error as LucetModuleError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /*
     * General #[from] implementations. */
    #[error("Anyhow error")]
    Any(#[from] anyhow::Error),
    #[error("Build error")]
    Build(#[from] parity_wasm::elements::Error),
    #[error("Clif module error")]
    ClifModuleError(#[from] ClifModuleError),
    #[error("Lucet Module error")]
    LucetModule(#[from] LucetModuleError),
    #[error("Lucet validation error")]
    LucetValidation(#[from] lucet_validate::Error),
    #[error("I/O error")]
    IOError(#[from] std::io::Error),
    // Attempts to use this in compilers.rs cause many failures in spectests.
    // #[error("Wasm validating parser error")]
    // WasmValidation(#[from] wasmparser::BinaryReaderError),
    #[error("Wat input")]
    WatInput(#[from] wabt::Error),
    /*
     * Cannot apply #[from] or #[source] to these error types due to missing traits. */
    #[error("Artifact error: {0:?}")]
    ArtifactError(failure::Error),
    #[error("Manifest error declaring {1}: {0:?}")]
    ManifestDeclaration(ArtifactError, String),
    #[error("Manifest error defining {1}: {0:?}")]
    ManifestDefinition(ArtifactError, String),
    #[error("Manifest error linking {1}: {0:?}")]
    ManifestLinking(failure::Error, String),
    #[error("Patcher error: {0:?}")]
    Patcher(wasmonkey::WError),
    #[error("Stack probe: {0:?}")]
    StackProbe(failure::Error),
    #[error("Table: {0:?}")]
    Table(failure::Error),
    #[error("Trap table error declaring {1}: {0:?}")]
    TrapTableDeclaration(ArtifactError, String),
    #[error("Error defining {0} writing the function trap table into the object: {0:?}")]
    TrapTableDefinition(ArtifactError, String),
    /*
     * And all the rest */
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
    #[error("Output error: {1}")]
    LoaderOutput(#[source] std::io::Error, String),
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
    #[error("Memory specs: {0}")]
    MemorySpecs(String),
    #[error("Metadata serializer; start index point to a non-function")]
    MetadataSerializer(#[source] ClifModuleError),
    #[error("Output: {0}")]
    Output(String),
    #[error("Output function: error writing function {1}")]
    OutputFunction(#[source] std::fmt::Error, String),
    #[error("Path error: {0}")]
    PathError(String),
    #[error("Read error: {0}")]
    ReadError(String),
    #[error("Signature error: {0}")]
    Signature(String),
    #[error("Error converting cranelift signature to wasm signature: {0}")]
    //    SignatureConversion(#[from] SignatureError), // TLC I wish I could do this...
    SignatureConversion(String),
    #[error("Table index is out of bounds: {0}")]
    TableIndexError(String),
    #[error("Translating module")]
    TranslatingModule,
    #[error("Translating lucet module")]
    TranslatingLucetModule(#[source] LucetModuleError),
    #[error("Translating cranelift module")]
    TranslatingClifModule(#[source] ClifModuleError),
    #[error("Translating cranelift wasm")]
    TranslatingClifWasm(#[source] ClifWasmError),
    #[error("Trap records are present for function {0} but the function does not exist.")]
    TrapRecord(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("host machine is not a supported target")]
    UnsupportedIsa(#[from] cranelift_codegen::isa::LookupError),
    #[error("Validation")]
    Validation,
    #[error("Writing clif file to file")]
    WritingClifFile(#[source] ClifModuleError),
}
