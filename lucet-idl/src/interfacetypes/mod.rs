mod lexer;
pub mod parser;
pub mod sexpr;

pub use parser::{DeclSyntax, ParseError, TopLevelSyntax};
use sexpr::SExprParseError;
pub use sexpr::SExprParser;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Fail)]
pub enum InterfaceTypesError {
    #[fail(display = "{}", _0)]
    SExpr(#[cause] SExprParseError),
    #[fail(display = "Invalid use statement \"{}\": {}", _1, _0)]
    UseInvalid(&'static str, String),
    #[fail(display = "When resolving use \"{}\": {}", _1, _0)]
    UseIo(#[cause] io::Error, String),
    #[fail(display = "cycle of use statements: {:?}", _0)]
    UseCycle(Vec<PathBuf>),
    #[fail(display = "{}", _0)]
    Parse(#[cause] ParseError),
}

impl From<ParseError> for InterfaceTypesError {
    fn from(e: ParseError) -> InterfaceTypesError {
        InterfaceTypesError::Parse(e)
    }
}

pub fn parse_witx<P: AsRef<Path>>(
    source_text: &str,
    search_path: P,
) -> Result<Vec<DeclSyntax>, InterfaceTypesError> {
    unimplemented!()
}

fn parse_toplevel(source_text: &str) -> Result<Vec<TopLevelSyntax>, InterfaceTypesError> {
    let mut sexpr_parser = SExprParser::new(source_text);
    let sexprs = sexpr_parser
        .match_sexprs()
        .map_err(InterfaceTypesError::SExpr)?;
    let top_levels = sexprs
        .iter()
        .map(|s| TopLevelSyntax::parse(s))
        .collect::<Result<Vec<TopLevelSyntax>, ParseError>>()?;
    Ok(top_levels)
}

fn resolve_uses(
    toplevel: Vec<TopLevelSyntax>,
    search_path: &Path,
    resolved: &mut Vec<PathBuf>,
) -> Result<Vec<DeclSyntax>, InterfaceTypesError> {
    let mut decls = Vec::new();

    for t in toplevel {
        match t {
            TopLevelSyntax::Decl(d) => decls.push(d),
            TopLevelSyntax::Use(u) => {
                let u_path = PathBuf::from(u.clone());
                if u_path.is_absolute() {
                    Err(InterfaceTypesError::UseInvalid(
                        "absolute path",
                        u.to_string(),
                    ))?;
                }
                let mut abs_path = PathBuf::from(search_path);
                abs_path.push(u_path);
                abs_path
                    .canonicalize()
                    .map_err(|e| InterfaceTypesError::UseIo(e, u.to_string()))?;
                if resolved.contains(&abs_path) {
                    Err(InterfaceTypesError::UseInvalid(
                        "loop of use statements",
                        u.to_string(),
                    ))?;
                }

                let source_text = fs::read_to_string(&abs_path)
                    .map_err(|e| InterfaceTypesError::UseIo(e, u.to_string()))?;
                let inner_toplevels = parse_toplevel(&source_text)?;

                resolved.push(abs_path);
                let inner_decls = resolve_uses(inner_toplevels, search_path, resolved)?;
                resolved.pop();
                decls.extend(inner_decls)
            }
        }
    }

    Ok(decls)
}
