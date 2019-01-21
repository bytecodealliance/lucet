use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

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
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }
    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for SpecTestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

#[derive(Fail, Debug, PartialEq, Eq)]
pub enum SpecTestErrorKind {
    #[fail(display = "Unsupported command")]
    UnsupportedCommand,
    #[fail(display = "Unexpected success")]
    UnexpectedSuccess,
    #[fail(display = "Unexpected failure")]
    UnexpectedFailure,
    #[fail(display = "Incorrect result")]
    IncorrectResult,
    #[fail(display = "Unsupported by lucetc")]
    UnsupportedLucetc,
}
