//TLC use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};
use thiserror::{Error};

#[derive(Debug)]
pub struct LucetcError {
    inner: Context<LucetcErrorKind>,
}

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

impl Fail for LucetcError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }
    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

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
