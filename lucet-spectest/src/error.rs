//use std::fmt::{self, Display};
use thiserror::Error;

/*
#[derive(Debug)]
pub struct SpecTestError {
    inner: Context<SpecTestErrorKind>,
}

impl From<Context<SpecTestErrorKind>> for SpecTestError {
    fn from(inner: Context<SpecTestErrorKind>) -> SpecTestError {
        SpecTestError { inner }
    }
}

impl From<SpecTestErrorKind> for SpecTestError {
    fn from(kind: SpecTestErrorKind) -> SpecTestError {
        SpecTestError {
            inner: Context::new(kind),
        }
    }
}

impl SpecTestError {
    pub fn get_context(&self) -> &SpecTestErrorKind {
        self.inner.get_context()
    }
}

impl Fail for SpecTestError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }
    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for SpecTestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
 */

#[derive(Debug, Error)]
pub enum Error {
    #[error("Parse error")]
    ParseError(#[from] wabt::script::Error),
    #[error("Read error")]
    ReadError(#[from] std::io::Error),
    #[error("Unsupported command: {0}")]
    UnsupportedCommand(String),
    #[error("Unexpected success")]
    UnexpectedSuccess,
    #[error("Unexpected failure: {0}")]
    UnexpectedFailure(String),
    #[error("Incorrect result: {0}")]
    IncorrectResult(String),
    #[error("Unsupported by lucetc")]
    UnsupportedLucetc,
}
