use super::parser::{DeclSyntax, ParseError, TopLevelSyntax};
use super::sexpr::SExprParser;
use super::InterfaceTypesError;
use std::fs;
use std::path::{Path, PathBuf};

pub fn parse_witx<P: AsRef<Path>>(i: P) -> Result<Vec<DeclSyntax>, InterfaceTypesError> {
    let i_buf = PathBuf::from(i.as_ref());
    let input_path = i_buf
        .canonicalize()
        .map_err(|e| InterfaceTypesError::Io(i_buf.clone(), e))?;
    let input = fs::read_to_string(&input_path)
        .map_err(|e| InterfaceTypesError::Io(input_path.clone(), e))?;

    let toplevel = parse_toplevel(&input, &input_path)?;
    let mut resolved = vec![input_path.clone()];
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
    used: &mut Vec<PathBuf>,
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

                // Include the decls from a use declaration only once
                // in a given toplevel. Same idea as #pragma once.
                if !used.contains(&abs_path) {
                    used.push(abs_path.clone());

                    let source_text = fs::read_to_string(&abs_path)
                        .map_err(|e| InterfaceTypesError::Io(abs_path.clone(), e))?;
                    let inner_toplevels = parse_toplevel(&source_text, &abs_path)?;

                    let inner_decls = resolve_uses(inner_toplevels, search_path, used)?;
                    decls.extend(inner_decls)
                }
            }
        }
    }

    Ok(decls)
}
