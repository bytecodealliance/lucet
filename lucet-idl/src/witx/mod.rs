pub mod ast;
mod lexer;
mod parser;
mod sexpr;
mod toplevel;
mod validate;

pub use parser::{DeclSyntax, ParseError};
pub use sexpr::SExprParseError;
pub use toplevel::parse_witx;
pub use validate::{validate, ValidationError};

use std::io;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Fail)]
pub enum WitxError {
    #[fail(display = "{}", _0)]
    SExpr(#[cause] SExprParseError),
    #[fail(display = "Invalid use statement at {:?}", _0)]
    UseInvalid(Location),
    #[fail(display = "in file {:?}: {}", _0, _1)]
    Parse(PathBuf, #[cause] ParseError),
    #[fail(display = "with file {:?}: {}", _0, _1)]
    Io(PathBuf, #[cause] io::Error),
}
