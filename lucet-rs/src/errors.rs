use crate::state::FaultDetails;
use failure::Fail;
use std::ffi;
use std::fmt;
use std::io;
use std::os::raw::c_void;

#[allow(dead_code)]
#[derive(Debug, Fail)]
pub enum LucetError {
    #[fail(display = "Runtime error: {}", _0)]
    RuntimeError(&'static str),
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    #[fail(display = "Incorrect usage: {}", _0)]
    UsageError(&'static str),
    #[fail(display = "Symbol not found: {}", _0)]
    SymbolNotFound(String),
    #[fail(display = "Function not found: ({}, {})", _0, _1)]
    FuncNotFound(u32, u32),
    #[fail(display = "Runtime fault: {:?}", _0)]
    RuntimeFault(FaultDetails),
    #[fail(display = "Runtime terminated")]
    RuntimeTerminated(TerminationDetails),
    #[fail(display = "IO Error: {}", _0)]
    Io(#[cause] io::Error),
    #[fail(display = "Parse error")]
    NulError(#[cause] ffi::NulError),
    #[fail(display = "NUL bytes in string")]
    ParseError,
    #[fail(display = "Unsupported")]
    Unsupported,
}

impl From<io::Error> for LucetError {
    fn from(e: io::Error) -> LucetError {
        LucetError::Io(e)
    }
}

impl From<ffi::NulError> for LucetError {
    fn from(e: ffi::NulError) -> LucetError {
        LucetError::NulError(e)
    }
}

pub struct TerminationDetails {
    pub details: *mut c_void,
}

// The void* underlying the termination details is totally unsafe. We're just going to have to deal
// with that until we rewrite the underlying library in Rust.
unsafe impl Send for TerminationDetails {}
unsafe impl Sync for TerminationDetails {}

impl fmt::Debug for TerminationDetails {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "TerminationDetails {{..}}")
    }
}
