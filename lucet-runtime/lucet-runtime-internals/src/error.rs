use crate::instance::{FaultDetails, TerminationDetails};
use failure::Fail;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Internal error: {}", _0)]
    InternalError(#[cause] failure::Error),
    #[fail(display = "Invalid argument: {}", _0)]
    InvalidArgument(&'static str),
    #[fail(display = "Symbol not found: {}", _0)]
    SymbolNotFound(String),
    #[fail(display = "Function not found: ({}, {})", _0, _1)]
    FuncNotFound(u32, u32),
    #[fail(display = "Runtime fault: {:?}", _0)]
    RuntimeFault(FaultDetails),
    #[fail(display = "Runtime terminated")]
    RuntimeTerminated(TerminationDetails),
    #[fail(display = "IO Error: {}", _0)]
    IoError(#[cause] std::io::Error),
}

impl From<failure::Error> for Error {
    fn from(e: failure::Error) -> Error {
        Error::InternalError(e)
    }
}

impl From<crate::context::Error> for Error {
    fn from(e: crate::context::Error) -> Error {
        Error::InternalError(e.into())
    }
}

impl From<nix::Error> for Error {
    fn from(e: nix::Error) -> Error {
        Error::InternalError(e.into())
    }
}

impl From<std::ffi::IntoStringError> for Error {
    fn from(e: std::ffi::IntoStringError) -> Error {
        Error::InternalError(e.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IoError(e)
    }
}

#[macro_export]
macro_rules! lucet_bail {
    ($e:expr) => {
        return Err(lucet_format_err!($e));
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(lucet_format_err!($fmt, $($arg)*));
    };
}

#[macro_export(local_inner_macros)]
macro_rules! lucet_ensure {
    ($cond:expr, $e:expr) => {
        if !($cond) {
            lucet_bail!($e);
        }
    };
    ($cond:expr, $fmt:expr, $($arg:tt)*) => {
        if !($cond) {
            lucet_bail!($fmt, $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! lucet_format_err {
    ($($arg:tt)*) => { $crate::error::Error::InternalError(failure::format_err!($($arg)*)) }
}
