use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

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
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }
    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for LucetcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[derive(Debug, Fail, PartialEq, Eq, Clone)]
pub enum LucetcErrorKind {
    #[fail(display = "Input")]
    Input,
    #[fail(display = "Validation")]
    Validation,
    #[fail(display = "Translating module")]
    TranslatingModule,
    #[fail(display = "Module data")]
    ModuleData,
    #[fail(display = "Metadata Serializer")] // specifically non-ModuleData; this will go away soon
    MetadataSerializer,
    #[fail(display = "Function Translation")]
    FunctionTranslation,
    #[fail(display = "Function Definition")]
    FunctionDefinition,
    #[fail(display = "Table")]
    Table,
    #[fail(display = "Memory Specs")]
    MemorySpecs,
    #[fail(display = "Output")]
    Output,
    #[fail(display = "Unsupported")]
    Unsupported,
}
