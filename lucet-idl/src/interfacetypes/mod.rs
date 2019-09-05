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
    #[fail(display = "cycle of use statements: {:?}", _0)]
    UseCycle(Vec<PathBuf>),
    #[fail(display = "in file {:?}: {}", _0, _1)]
    Parse(PathBuf, #[cause] ParseError),
    #[fail(display = "with file {:?}: {}", _0, _1)]
    Io(PathBuf, #[cause] io::Error),
}

pub fn parse_witx<P: AsRef<Path>>(input_path: P) -> Result<Vec<DeclSyntax>, InterfaceTypesError> {
    let input_path = input_path.as_ref();
    let input = fs::read_to_string(input_path)
        .map_err(|e| InterfaceTypesError::Io(input_path.into(), e))?;

    let toplevel = parse_toplevel(&input, input_path)?;
    let mut resolved = vec![input_path.into()];
    let search_path = input_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from("."));
    resolve_uses(toplevel, &search_path, &mut resolved)
}

fn parse_toplevel(
    source_text: &str,
    file_path: &Path,
) -> Result<Vec<TopLevelSyntax>, InterfaceTypesError> {
    let mut sexpr_parser = SExprParser::new(source_text);
    let sexprs = sexpr_parser
        .match_sexprs()
        .map_err(InterfaceTypesError::SExpr)?;
    let top_levels = sexprs
        .iter()
        .map(|s| TopLevelSyntax::parse(s))
        .collect::<Result<Vec<TopLevelSyntax>, ParseError>>()
        .map_err(|e| InterfaceTypesError::Parse(file_path.into(), e))?;
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
                abs_path.push(u_path.clone());
                abs_path
                    .canonicalize()
                    .map_err(|e| InterfaceTypesError::Io(u_path, e))?;
                if resolved.contains(&abs_path) {
                    resolved.push(abs_path.clone());
                    Err(InterfaceTypesError::UseCycle(resolved.clone()))?;
                }

                let source_text = fs::read_to_string(&abs_path)
                    .map_err(|e| InterfaceTypesError::Io(abs_path.clone(), e))?;
                let inner_toplevels = parse_toplevel(&source_text, &abs_path)?;

                resolved.push(abs_path);
                let inner_decls = resolve_uses(inner_toplevels, search_path, resolved)?;
                resolved.pop();
                decls.extend(inner_decls)
            }
        }
    }

    Ok(decls)
}
