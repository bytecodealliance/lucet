use parity_wasm::elements;
use std::io;

#[allow(dead_code)]
#[derive(Debug, Fail)]
pub enum WError {
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    #[fail(display = "Incorrect usage: {}", _0)]
    UsageError(&'static str),
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
    #[fail(display = "{}", _0)]
    WAsmError(#[cause] elements::Error),
    #[fail(display = "Parse error")]
    ParseError,
    #[fail(display = "Unsupported")]
    Unsupported,
}

impl From<io::Error> for WError {
    fn from(e: io::Error) -> WError {
        WError::Io(e)
    }
}

impl From<elements::Error> for WError {
    fn from(e: elements::Error) -> WError {
        WError::WAsmError(e)
    }
}
