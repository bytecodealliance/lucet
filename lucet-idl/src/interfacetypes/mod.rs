pub mod ast;
mod lexer;
mod parser;
mod sexpr;
mod toplevel;

pub use parser::{DeclSyntax, ParseError};
pub use sexpr::SExprParseError;
pub use toplevel::parse_witx;

use std::io;
use std::path::PathBuf;

#[derive(Debug, Fail)]
pub enum InterfaceTypesError {
    #[fail(display = "{}", _0)]
    SExpr(#[cause] SExprParseError),
    #[fail(display = "Invalid use statement \"{}\": {}", _1, _0)]
    UseInvalid(&'static str, String),
    #[fail(display = "in file {:?}: {}", _0, _1)]
    Parse(PathBuf, #[cause] ParseError),
    #[fail(display = "with file {:?}: {}", _0, _1)]
    Io(PathBuf, #[cause] io::Error),
}
