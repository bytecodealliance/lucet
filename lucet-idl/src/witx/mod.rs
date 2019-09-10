pub mod ast;
mod lexer;
mod parser;
mod sexpr;
mod toplevel;
mod validate;

pub use ast::Document;
pub use parser::{DeclSyntax, ParseError};
pub use sexpr::SExprParseError;
pub use toplevel::parse_witx;
pub use validate::{validate_document, ValidationError};

use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Fail)]
pub enum WitxError {
    #[fail(display = "with file {:?}: {}", _0, _1)]
    Io(PathBuf, #[cause] io::Error),
    #[fail(display = "{}", _0)]
    SExpr(#[cause] SExprParseError),
    #[fail(display = "{}", _0)]
    Parse(#[cause] ParseError),
    #[fail(display = "{}", _0)]
    Validation(#[cause] ValidationError),
}

pub fn load_witx<P: AsRef<Path>>(path: P) -> Result<Document, WitxError> {
    let parsed_decls = parse_witx(path)?;
    validate_document(&parsed_decls).map_err(WitxError::Validation)
}
