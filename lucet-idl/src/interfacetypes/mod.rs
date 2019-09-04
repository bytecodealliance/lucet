mod lexer;
pub mod parser;
pub mod sexpr;

use crate::IDLError;
pub use parser::{DeclSyntax, ParseError};
pub use sexpr::SExprParser;

pub fn parse<'a>(
    source_text: &'a str,
) -> Result<Vec<Result<DeclSyntax<'a>, ParseError>>, IDLError> {
    let mut sexpr_parser = SExprParser::new(source_text);
    let sexprs = sexpr_parser
        .match_sexprs()
        .map_err(|e| IDLError::InterfaceTypes(e.into()))?;

    Ok(sexprs
        .iter()
        .map(|sexpr| DeclSyntax::parse(sexpr))
        .collect::<Vec<Result<_, _>>>())
}
