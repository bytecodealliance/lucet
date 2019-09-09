use super::parser::{DeclSyntax, ParseError, TopLevelSyntax};
use super::sexpr::SExprParser;
use super::WitxError;
use std::fs;
use std::path::{Path, PathBuf};

pub fn parse_witx<P: AsRef<Path>>(i: P) -> Result<Vec<DeclSyntax>, WitxError> {
    parse_witx_with(i, &Filesystem)
}

trait WitxIo {
    fn fgets(&self, path: &Path) -> Result<String, WitxError>;
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, WitxError>;
}

struct Filesystem;

impl WitxIo for Filesystem {
    fn fgets(&self, path: &Path) -> Result<String, WitxError> {
        fs::read_to_string(path).map_err(|e| WitxError::Io(path.to_path_buf(), e))
    }
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, WitxError> {
        path.canonicalize()
            .map_err(|e| WitxError::Io(path.to_path_buf(), e))
    }
}

fn parse_witx_with<P: AsRef<Path>>(
    i: P,
    witxio: &dyn WitxIo,
) -> Result<Vec<DeclSyntax>, WitxError> {
    let i_buf = PathBuf::from(i.as_ref());
    let input_path = witxio.canonicalize(&i_buf)?;

    let input = witxio.fgets(&input_path)?;

    let toplevel = parse_toplevel(&input, &input_path)?;
    let mut resolved = vec![input_path.clone()];
    let search_path = input_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from("."));
    resolve_uses(toplevel, &search_path, &mut resolved, witxio)
}

fn parse_toplevel(source_text: &str, file_path: &Path) -> Result<Vec<TopLevelSyntax>, WitxError> {
    let mut sexpr_parser = SExprParser::new(source_text, file_path);
    let sexprs = sexpr_parser.match_sexprs().map_err(WitxError::SExpr)?;
    let top_levels = sexprs
        .iter()
        .map(|s| TopLevelSyntax::parse(s))
        .collect::<Result<Vec<TopLevelSyntax>, ParseError>>()
        .map_err(|e| WitxError::Parse(file_path.into(), e))?;
    Ok(top_levels)
}

fn resolve_uses(
    toplevel: Vec<TopLevelSyntax>,
    search_path: &Path,
    used: &mut Vec<PathBuf>,
    witxio: &dyn WitxIo,
) -> Result<Vec<DeclSyntax>, WitxError> {
    let mut decls = Vec::new();

    for t in toplevel {
        match t {
            TopLevelSyntax::Decl(d) => decls.push(d),
            TopLevelSyntax::Use(u) => {
                let u_path = PathBuf::from(&u.name);
                if u_path.is_absolute() {
                    Err(WitxError::UseInvalid(u.location.clone()))?;
                }
                let mut abs_path = PathBuf::from(search_path);
                abs_path.push(u_path.clone());
                let abs_path = witxio.canonicalize(&abs_path)?;

                // Include the decls from a use declaration only once
                // in a given toplevel. Same idea as #pragma once.
                if !used.contains(&abs_path) {
                    used.push(abs_path.clone());

                    let source_text = witxio.fgets(&abs_path)?;
                    let inner_toplevels = parse_toplevel(&source_text, &abs_path)?;

                    let inner_decls = resolve_uses(inner_toplevels, search_path, used, witxio)?;
                    decls.extend(inner_decls)
                }
            }
        }
    }

    Ok(decls)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    struct MockFs {
        map: HashMap<&'static str, &'static str>,
    }

    impl MockFs {
        pub fn new(strings: Vec<(&'static str, &'static str)>) -> Self {
            MockFs {
                map: strings.into_iter().collect(),
            }
        }
    }

    impl WitxIo for MockFs {
        fn fgets(&self, path: &Path) -> Result<String, WitxError> {
            if let Some(entry) = self.map.get(path.to_str().unwrap()) {
                Ok(entry.to_string())
            } else {
                Err(WitxError::Io(path.to_path_buf(), panic!("idk!!!")))
            }
        }
        fn canonicalize(&self, path: &Path) -> Result<PathBuf, WitxError> {
            Ok(PathBuf::from(path))
        }
    }

    #[test]
    fn empty() {
        assert_eq!(
            parse_witx_with(&Path::new("/a"), &MockFs::new(vec![("/a", ";; empty")]))
                .expect("parse"),
            Vec::new(),
        );
    }

    #[test]
    fn one_include() {
        assert_eq!(
            parse_witx_with(
                &Path::new("/a"),
                &MockFs::new(vec![("/a", "(use \"b\")"), ("/b", ";; empty")])
            )
            .expect("parse"),
            Vec::new(),
        );
    }
}
