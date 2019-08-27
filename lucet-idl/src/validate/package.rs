use super::module::module_from_declarations;
use crate::error::ValidationError;
use crate::parser::PackageDecl;
use crate::prelude::std_module;
use crate::repr::{ModuleIx, ModuleRepr, Package};
use crate::Location;
use cranelift_entity::PrimaryMap;
use std::collections::HashMap;

pub fn package_from_declarations(
    package_decls: &[PackageDecl],
) -> Result<Package, ValidationError> {
    let mut pkg = PackageBuilder::new();
    for decl in package_decls {
        match decl {
            PackageDecl::Module {
                name,
                location,
                decls,
            } => {
                let module_ix = pkg.introduce_name(name, *location)?;
                let module_repr = module_from_declarations(pkg.repr(), module_ix, &decls)?;
                pkg.define_module(module_ix, module_repr);
            }
        }
    }
    Ok(pkg.build())
}

pub struct PackageBuilder {
    name_decls: HashMap<String, (ModuleIx, Option<Location>)>,
    repr: Package,
}

impl PackageBuilder {
    pub fn new() -> Self {
        let mut repr = Package {
            names: PrimaryMap::new(),
            modules: PrimaryMap::new(),
        };
        let mut name_decls = HashMap::new();
        repr.names.push("std".to_owned());
        let base_ix = repr.modules.push(std_module());
        name_decls.insert("std".to_owned(), (base_ix, None));

        Self { name_decls, repr }
    }

    pub fn introduce_name(
        &mut self,
        name: &str,
        location: Location,
    ) -> Result<ModuleIx, ValidationError> {
        if let Some((_, prev_loc)) = self.name_decls.get(name) {
            match prev_loc {
                Some(prev_loc) => {
                    Err(ValidationError::NameAlreadyExists {
                        name: name.to_owned(),
                        at_location: location,
                        previous_location: *prev_loc,
                    })?;
                }
                None => {
                    Err(ValidationError::Syntax {
                        expected: "non-reserved module name",
                        location: location,
                    })?;
                }
            }
        }
        let ix = self.repr.names.push(name.to_owned());
        self.name_decls
            .insert(name.to_owned(), (ix, Some(location)));
        Ok(ix)
    }

    pub fn repr(&self) -> &Package {
        &self.repr
    }

    pub fn define_module(&mut self, ix: ModuleIx, mod_repr: ModuleRepr) {
        assert!(self.repr.names.is_valid(ix));
        let pushed_ix = self.repr.modules.push(mod_repr);
        assert_eq!(ix, pushed_ix);
    }

    pub fn build(self) -> Package {
        self.repr
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parser::Parser;
    use crate::Package;

    fn pkg_(syntax: &str) -> Result<Package, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_package_decls().expect("parses");
        package_from_declarations(&decls)
    }

    #[test]
    fn one_empty_mod() {
        let pkg = pkg_("mod empty {}").expect("valid package");
        let empty = pkg.module("empty").expect("mod empty exists");
        assert_eq!(empty.name(), "empty");
        assert_eq!(empty.datatypes().collect::<Vec<_>>().len(), 0);
        assert_eq!(empty.functions().collect::<Vec<_>>().len(), 0);
    }

    #[test]
    fn multiple_empty_mod() {
        let pkg = pkg_("mod empty1 {} mod empty2{}mod\nempty3{//\n}").expect("valid package");
        let _ = pkg.module("empty1").expect("mod empty1 exists");
        let _ = pkg.module("empty2").expect("mod empty2 exists");
        let _ = pkg.module("empty3").expect("mod empty3 exists");
    }

    #[test]
    fn mod_with_a_type() {
        let pkg = pkg_("mod foo { type bar = u8; }").expect("valid package");
        let foo = pkg.module("foo").expect("mod foo exists");
        let _bar = foo.datatype("bar").expect("foo::bar exists");
    }

    #[test]
    fn no_mod_std_name() {
        let err = pkg_("mod std {}").err().expect("error package");
        assert_eq!(
            err,
            ValidationError::Syntax {
                expected: "non-reserved module name",
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn no_mod_duplicate_name() {
        let err = pkg_("mod foo {}\nmod foo {}").err().expect("error package");
        assert_eq!(
            err,
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );
    }
}
