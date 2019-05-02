use crate::{package, parser};
use std::io;

#[allow(dead_code)]
#[derive(Debug, Fail)]
pub enum IDLError {
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    #[fail(display = "Incorrect usage: {}", _0)]
    UsageError(&'static str),
    #[fail(display = "{}", _0)]
    ParseError(#[cause] parser::ParseError),
    #[fail(display = "{}", _0)]
    ValidationError(#[cause] package::ValidationError),
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
}

impl From<io::Error> for IDLError {
    fn from(e: io::Error) -> Self {
        IDLError::Io(e)
    }
}

impl From<parser::ParseError> for IDLError {
    fn from(e: parser::ParseError) -> Self {
        IDLError::ParseError(e)
    }
}

impl From<package::ValidationError> for IDLError {
    fn from(e: package::ValidationError) -> Self {
        IDLError::ValidationError(e)
    }
}
