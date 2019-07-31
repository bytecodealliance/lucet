#![allow(unused)]
use super::datatypes::DatatypeModuleBuilder;
use super::function::FunctionModuleBuilder;
use super::names::ModNamesBuilder;
use crate::env::cursor::Package;
use crate::env::repr::{DatatypeIx, FuncIx, ModuleIx, ModuleRepr, PackageRepr};
use crate::error::ValidationError;
use crate::parser::SyntaxDecl;
use crate::types::Location;
use std::collections::HashMap;

pub fn module_from_declarations(
    env: &PackageRepr,
    ix: ModuleIx,
    decls: &[SyntaxDecl],
) -> Result<ModuleRepr, ValidationError> {
    // First, we need to declare names of all the declarations
    let mut names = ModNamesBuilder::new(ix);
    for decl in decls.iter() {
        match decl {
            SyntaxDecl::Struct { name, location, .. }
            | SyntaxDecl::Enum { name, location, .. }
            | SyntaxDecl::Alias { name, location, .. } => {
                names.introduce_datatype(name, location)?;
            }
            SyntaxDecl::Function { name, location, .. } => {
                names.introduce_function(name, location)?;
            }
            SyntaxDecl::Module { .. } => unreachable!(),
        }
    }

    // Datatypes are defined in terms of the parent environment:
    let data_env = Package::new(env);
    let mut datatypes_builder = DatatypeModuleBuilder::new(data_env, &names);

    // Then, we can define each datatype
    for decl in decls.iter() {
        match decl {
            SyntaxDecl::Struct {
                name,
                location,
                members,
            } => {
                datatypes_builder.introduce_struct(name, members, location)?;
            }
            SyntaxDecl::Enum {
                name,
                location,
                variants,
            } => {
                datatypes_builder.introduce_enum(name, variants, location)?;
            }
            SyntaxDecl::Alias {
                name,
                location,
                what,
            } => {
                datatypes_builder.introduce_alias(name, what, location)?;
            }
            _ => {}
        }
    }

    // Finalize the datatypes - ensure finite, calculate layout information:
    let datatypes_module = datatypes_builder.build()?;

    // Cons these datatypes onto the packagerepr, then create a Module cursor
    let mut funcs_env_repr = env.clone();
    funcs_env_repr
        .modules
        .push(ModuleRepr::from_datatypes(datatypes_module.clone()));
    let funcs_env = Package::new(&funcs_env_repr).module_by_ix(ix).unwrap();

    // Now we can define each function:
    let mut funcs_builder = FunctionModuleBuilder::new(funcs_env, &names);

    for decl in decls {
        if let SyntaxDecl::Function {
            name,
            args,
            rets,
            bindings,
            location,
        } = decl
        {
            funcs_builder.introduce_func(name, args, rets, bindings, location)?;
        }
    }

    let funcs_module = funcs_builder.build();

    Ok(ModuleRepr {
        datatypes: datatypes_module,
        funcs: funcs_module,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::cursor::{DatatypeVariant, Module};
    use crate::env::prelude::base_package;
    use crate::env::MemArea;
    use crate::parser::Parser;
    fn mod_syntax(syntax: &str) -> Result<PackageRepr, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_decls().expect("parses");

        let mut pkg = base_package();
        let mod_ix = pkg.names.push("mod".to_owned());
        let module_repr = module_from_declarations(&pkg, mod_ix, &decls)?;
        pkg.modules.push(module_repr);
        Ok(pkg)
    }

    #[test]
    fn structs_basic() {
        assert!(mod_syntax("struct foo { a: i32}").is_ok());
        assert!(mod_syntax("struct foo { a: i32, b: f32 }").is_ok());
    }

    #[test]
    fn struct_two_atoms() {
        let pkg_r = mod_syntax("struct foo { a: i32, b: f32 }").expect("valid");
        let module = Package::new(&pkg_r).module("mod").unwrap();
        let foo = module.datatype("foo").expect("foo datatype defined");
        assert_eq!(foo.mem_size(), 8);
        assert_eq!(foo.mem_align(), 4);
        match foo.variant() {
            DatatypeVariant::Struct(s) => {
                assert_eq!(s.members().collect::<Vec<_>>().len(), 2);
                let a = s.member("a").expect("get member a");
                assert_eq!(a.name(), "a");
                assert_eq!(a.type_().name(), "i32");
                assert_eq!(a.offset(), 0);

                let b = s.member("b").expect("get member b");
                assert_eq!(b.name(), "b");
                assert_eq!(b.type_().name(), "f32");
                assert_eq!(b.offset(), 4);
            }
            _ => panic!("foo is a struct!"),
        }
    }

    #[test]
    fn struct_prev_definition() {
        // Refer to a struct defined previously:
        assert!(mod_syntax("struct foo { a: i32, b: f64 } struct bar { a: foo }").is_ok());
    }

    #[test]
    fn struct_next_definition() {
        // Refer to a struct defined afterwards:
        assert!(mod_syntax("struct foo { a: i32, b: bar} struct bar { a: i32 }").is_ok());
    }

    #[test]
    fn struct_self_referential() {
        // Refer to itself
        let e = mod_syntax("struct list { next: list, thing: i32 }");
        assert!(e.is_err());
        assert_eq!(
            e.err().unwrap(),
            ValidationError::Infinite {
                name: "list".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn struct_empty() {
        // No members
        assert_eq!(
            mod_syntax("struct foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn struct_duplicate_member() {
        // Duplicate member in struct
        assert_eq!(
            mod_syntax("struct foo { \na: i32, \na: f64}")
                .err()
                .unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );
    }

    #[test]
    fn struct_duplicate_definition() {
        // Duplicate definition of struct
        assert_eq!(
            mod_syntax("struct foo { a: i32 }\nstruct foo { a: i32 } ")
                .err()
                .unwrap(),
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn struct_undeclared_member() {
        // Refer to type that is not declared
        assert_eq!(
            mod_syntax("struct foo { \nb: bar }").err().unwrap(),
            ValidationError::NameNotFound {
                name: "bar".to_owned(),
                use_location: Location { line: 2, column: 3 },
            }
        );
    }

    #[test]
    fn enums() {
        assert!(mod_syntax("enum foo { a }").is_ok());
        assert!(mod_syntax("enum foo { a, b }").is_ok());

        {
            let pkg_repr = mod_syntax("enum foo { a, b }").expect("valid syntax");
            let m = Package::new(&pkg_repr).module("mod").expect("get module");
            let foo = m.datatype("foo").expect("get foo");
            match foo.variant() {
                DatatypeVariant::Enum(e) => {
                    assert_eq!(e.variants().collect::<Vec<_>>().len(), 2);
                    let a = e.variant("a").expect("variant a exists");
                    assert_eq!(a.name(), "a");
                    assert_eq!(a.value(), 0);
                    let b = e.variant("b").expect("variant b exists");
                    assert_eq!(b.name(), "b");
                    assert_eq!(b.value(), 1);
                }
                _ => panic!("foo is an enum!"),
            }
        }

        // No members
        assert_eq!(
            mod_syntax("enum foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        // Duplicate member in enum
        assert_eq!(
            mod_syntax("enum foo { \na,\na }").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );

        // Duplicate definition of enum
        assert_eq!(
            mod_syntax("enum foo { a }\nenum foo { a } ").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn aliases() {
        assert!(mod_syntax("type foo = i32;").is_ok());
        assert!(mod_syntax("type foo = f64;").is_ok());
        assert!(mod_syntax("type foo = u8;").is_ok());
        assert!(mod_syntax("type link = u32;\nstruct list { next: link, thing: i32 }").is_ok());

        let pkg_repr = mod_syntax("type foo = bar;\nenum bar { a }").expect("valid");
        let m = Package::new(&pkg_repr).module("mod").expect("get module");
        let foo = m.datatype("foo").expect("get foo");

        match foo.variant() {
            DatatypeVariant::Alias(a) => {
                assert_eq!(a.name(), "foo");
                assert_eq!(a.to().name(), "bar");
            }
            _ => panic!("foo is an alias"),
        }
    }

    #[test]
    fn infinite() {
        assert_eq!(
            mod_syntax("type foo = bar;\ntype bar = foo;")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            mod_syntax("type foo = bar;\nstruct bar { a: foo }")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            mod_syntax("type foo = bar;\nstruct bar { a: baz }\nstruct baz { c: i32, e: foo }")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );
    }

}
