use crate::error::ValidationError;
use crate::module::Module;
use crate::parser::SyntaxDecl;
use crate::types::{Ident, Location, Name};
use heck::SnakeCase;
use lucet_module_data::bindings::Bindings;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Package {
    pub names: Vec<Name>,
    pub modules: HashMap<Ident, Module>,
}

impl Package {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            modules: HashMap::new(),
        }
    }

    fn introduce_name(
        &mut self,
        name: &str,
        location: &Location,
    ) -> Result<Ident, ValidationError> {
        if let Some(existing) = self.id_for_name(&name) {
            let prev = self
                .names
                .get(existing.0)
                .expect("lookup told us name exists");
            Err(ValidationError::NameAlreadyExists {
                name: name.to_owned(),
                at_location: *location,
                previous_location: prev.location,
            })
        } else {
            let id = self.names.len();
            self.names.push(Name {
                name: name.to_owned(),
                location: *location,
            });
            Ok(Ident(id))
        }
    }

    fn id_for_name(&self, name: &str) -> Option<Ident> {
        for (id, n) in self.names.iter().enumerate() {
            if n.name == name {
                return Some(Ident(id));
            }
        }
        None
    }

    fn define_module(&mut self, id: Ident, mod_: Module) {
        if let Some(prev_def) = self.modules.insert(id, mod_) {
            panic!("id {} already defined: {:?}", id, prev_def)
        }
    }

    pub fn from_declarations(decls: &[SyntaxDecl]) -> Result<Package, ValidationError> {
        let mut pkg = Self::new();
        let mut idents: Vec<Ident> = Vec::new();
        for decl in decls {
            match decl {
                SyntaxDecl::Module { name, location, .. } => {
                    idents.push(pkg.introduce_name(name, location)?);
                }
                _ => Err(ValidationError::Syntax {
                    expected: "module",
                    location: *decl.location(),
                })?,
            }
        }

        for (decl, id) in decls.iter().zip(&idents) {
            match decl {
                SyntaxDecl::Module { decls, name, .. } => {
                    let binding_prefix = "__".to_owned() + &name.to_snake_case();
                    pkg.define_module(
                        *id,
                        Module::from_declarations(decls, name.clone(), binding_prefix)?,
                    );
                }
                _ => unreachable!(),
            }
        }

        Ok(pkg)
    }

    pub fn bindings(&self) -> Bindings {
        let b: HashMap<String, HashMap<String, String>> = self
            .modules
            .iter()
            .map(|(_, m)| (m.module_name.clone(), m.func_bindings()))
            .collect();
        Bindings::new(b)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parser::Parser;
    use crate::types::{AliasDataType, AtomType, DataType, DataTypeRef, DataTypeVariant};
    fn pkg_(syntax: &str) -> Result<Package, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_decls().expect("parses");
        Package::from_declarations(&decls)
    }

    #[test]
    fn trivial() {
        let pkg = pkg_("mod empty {}").expect("valid package");
        assert_eq!(
            pkg.names,
            vec![Name {
                name: "empty".to_owned(),
                location: Location { line: 1, column: 0 }
            }]
        );
        assert_eq!(
            pkg.modules,
            vec![(
                Ident(0),
                Module {
                    names: Vec::new(),
                    data_types: HashMap::new(),
                    data_type_ordering: Vec::new(),
                    funcs: HashMap::new(),
                    module_name: "empty".to_owned(),
                    binding_prefix: "__empty".to_owned(),
                }
            )]
            .into_iter()
            .collect::<HashMap<Ident, Module>>()
        );
    }

    #[test]
    fn multiple_empty_mods() {
        let pkg = pkg_("mod empty1 {} mod empty2{}mod\nempty3{//\n}").expect("valid package");
        assert_eq!(
            pkg.names,
            vec![
                Name {
                    name: "empty1".to_owned(),
                    location: Location { line: 1, column: 0 }
                },
                Name {
                    name: "empty2".to_owned(),
                    location: Location {
                        line: 1,
                        column: 14
                    }
                },
                Name {
                    name: "empty3".to_owned(),
                    location: Location {
                        line: 1,
                        column: 26
                    }
                }
            ]
        );
        assert_eq!(
            pkg.modules,
            vec![
                (
                    Ident(0),
                    Module {
                        names: Vec::new(),
                        data_types: HashMap::new(),
                        data_type_ordering: Vec::new(),
                        funcs: HashMap::new(),
                        module_name: "empty1".to_owned(),
                        binding_prefix: "__empty1".to_owned(),
                    }
                ),
                (
                    Ident(1),
                    Module {
                        names: Vec::new(),
                        data_types: HashMap::new(),
                        data_type_ordering: Vec::new(),
                        funcs: HashMap::new(),
                        module_name: "empty2".to_owned(),
                        binding_prefix: "__empty2".to_owned(),
                    }
                ),
                (
                    Ident(2),
                    Module {
                        names: Vec::new(),
                        data_types: HashMap::new(),
                        data_type_ordering: Vec::new(),
                        funcs: HashMap::new(),
                        module_name: "empty3".to_owned(),
                        binding_prefix: "__empty3".to_owned(),
                    }
                )
            ]
            .into_iter()
            .collect::<HashMap<Ident, Module>>()
        );
    }

    #[test]
    fn mod_with_a_type() {
        let pkg = pkg_("mod foo { type bar = u8; }").expect("valid package");
        assert_eq!(
            pkg.names,
            vec![Name {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 }
            }]
        );
        assert_eq!(
            pkg.modules,
            vec![(
                Ident(0),
                Module {
                    names: vec![Name {
                        name: "bar".to_owned(),
                        location: Location {
                            line: 1,
                            column: 10
                        }
                    }],
                    funcs: HashMap::new(),
                    data_types: vec![(
                        Ident(0),
                        DataType {
                            variant: DataTypeVariant::Alias(AliasDataType {
                                to: DataTypeRef::Atom(AtomType::U8)
                            }),
                            repr_size: 1,
                            align: 1,
                        }
                    )]
                    .into_iter()
                    .collect::<HashMap<Ident, DataType>>(),
                    data_type_ordering: vec![Ident(0)],
                    module_name: "foo".to_owned(),
                    binding_prefix: "__foo".to_owned(),
                }
            )]
            .into_iter()
            .collect::<HashMap<Ident, Module>>()
        );
    }

    #[test]
    fn no_mod_duplicate_name() {
        pkg_("mod foo {} mod foo {}").err().expect("error package");
    }

    #[test]
    fn no_mod_in_mod() {
        pkg_("mod foo { mod bar { }}").err().expect("error package");
        pkg_("mod foo { enum whatever {} mod bar { }}")
            .err()
            .expect("error package");
    }

    #[test]
    fn no_top_level_types() {
        pkg_("mod foo { } enum bar {}")
            .err()
            .expect("error package");
    }
}
