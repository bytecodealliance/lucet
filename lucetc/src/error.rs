//TLC use failure::{Backtrace, Context, Fail};
use anyhow::Context;
use cranelift_module::ModuleError as ClifModuleError;
use std::fmt::{self, Display};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Input")]
    Input,
    #[error("Validation")]
    Validation,
    #[error("Translating module")]
    TranslatingModule,
    #[error("Module data")]
    ModuleData,
    #[error("Metadata serializer; start index pointed to a non-function")]
    // specifically non-ModuleData; this will go away soon
    MetadataSerializer,
    #[error("Function translation error in {symbol}: {source:?}")]
    FunctionTranslation {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Function definition error in {symbol}: {source:?}")]
    FunctionDefinition {
        symbol: String,
        #[source]
        source: ClifModuleError,
    },
    #[error("Table")]
    Table,
    #[error("Memory specs")]
    MemorySpecs,
    #[error("Output")]
    Output,
    #[error("Signature")]
    Signature,
    #[error("Unsupported")]
    Unsupported,
}

// TLC: I think I can derive these froms with thiserror.
impl From<Context<LucetcErrorKind>> for LucetcError {
    fn from(inner: Context<LucetcErrorKind>) -> LucetcError {
        LucetcError { inner }
    }
}

impl From<LucetcErrorKind> for LucetcError {
    fn from(kind: LucetcErrorKind) -> LucetcError {
        LucetcError {
            inner: Context::new(kind),
        }
    }
}

impl LucetcError {
    pub fn get_context(&self) -> &LucetcErrorKind {
        self.inner.get_context()
    }
}

/*
impl Fail for LucetcError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }
    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}
*/

impl Display for LucetcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum LucetcErrorKind {
    #[error("Input")]
    Input,
    #[error("Validation")]
    Validation,
    #[error("Translating module")]
    TranslatingModule,
    #[error("Module data")]
    ModuleData,
    #[error("Metadata Serializer")] // specifically non-ModuleData; this will go away soon
    MetadataSerializer,
    #[error("Function Translation")]
    FunctionTranslation,
    #[error("Function Definition")]
    FunctionDefinition,
    #[error("Table")]
    Table,
    #[error("Memory Specs")]
    MemorySpecs,
    #[error("Output")]
    Output,
    #[error("Signature")]
    Signature,
    #[error("Unsupported")]
    Unsupported,
}
