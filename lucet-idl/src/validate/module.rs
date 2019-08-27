use super::datatypes::DatatypeModuleBuilder;
use super::function::FunctionModuleBuilder;
use super::names::ModNamesBuilder;
use crate::parser::ModuleDecl;
use crate::repr::{ModuleIx, ModuleRepr};
use crate::{Package, ValidationError};

pub fn module_from_declarations(
    env: &Package,
    ix: ModuleIx,
    decls: &[ModuleDecl],
) -> Result<ModuleRepr, ValidationError> {
    // First, we need to declare names of all the declarations
    let mut names = ModNamesBuilder::new(ix);
    for decl in decls.iter() {
        match decl {
            ModuleDecl::Struct { name, location, .. }
            | ModuleDecl::Enum { name, location, .. }
            | ModuleDecl::Alias { name, location, .. } => {
                names.introduce_datatype(name, location)?;
            }
            ModuleDecl::Function { name, location, .. } => {
                names.introduce_function(name, location)?;
            }
        }
    }

    // Datatypes are defined in terms of the parent environment:
    let mut datatypes_builder = DatatypeModuleBuilder::new(env, &names);

    // Then, we can define each datatype
    for decl in decls.iter() {
        match decl {
            ModuleDecl::Struct {
                name,
                location,
                members,
            } => {
                datatypes_builder.introduce_struct(name, members, *location)?;
            }
            ModuleDecl::Enum {
                name,
                location,
                variants,
            } => {
                datatypes_builder.introduce_enum(name, variants, *location)?;
            }
            ModuleDecl::Alias {
                name,
                location,
                what,
            } => {
                datatypes_builder.introduce_alias(name, what, *location)?;
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
    let funcs_env = funcs_env_repr.module_by_ix(ix).unwrap();

    // Now we can define each function:
    let mut funcs_builder = FunctionModuleBuilder::new(funcs_env, &names);

    for decl in decls {
        if let ModuleDecl::Function {
            name,
            args,
            rets,
            bindings,
            location,
        } = decl
        {
            funcs_builder.introduce_func(name, args, rets, bindings, *location)?;
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
    use crate::parser::Parser;
    use crate::validate::package::PackageBuilder;
    use crate::{BindingDirection, DatatypeVariant, Location, MemArea, ParamPosition};
    fn mod_syntax(syntax: &str) -> Result<Package, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_module_decls().expect("parses");

        let mut pkg_builder = PackageBuilder::new();
        let mod_ix = pkg_builder
            .introduce_name("mod", Location { line: 0, column: 0 })
            .expect("declare name ok");
        let module_repr = module_from_declarations(pkg_builder.repr(), mod_ix, &decls)?;
        pkg_builder.define_module(mod_ix, module_repr);
        Ok(pkg_builder.build())
    }

    #[test]
    fn structs_basic() {
        assert!(mod_syntax("struct foo { a: i32}").is_ok());
        assert!(mod_syntax("struct foo { a: i32, b: f32 }").is_ok());
    }

    #[test]
    fn struct_two_atoms() {
        let pkg_r = mod_syntax("struct foo { a: i32, b: f32 }").expect("valid");
        let module = pkg_r.module("mod").unwrap();
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
            let pkg = mod_syntax("enum foo { a, b }").expect("valid syntax");
            let m = pkg.module("mod").expect("get module");
            let foo = m.datatype("foo").expect("get foo");
            match foo.variant() {
                DatatypeVariant::Enum(e) => {
                    assert_eq!(e.variants().collect::<Vec<_>>().len(), 2);
                    let a = e.variant("a").expect("variant a exists");
                    assert_eq!(a.name(), "a");
                    assert_eq!(a.index(), 0);
                    let b = e.variant("b").expect("variant b exists");
                    assert_eq!(b.name(), "b");
                    assert_eq!(b.index(), 1);
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

        let pkg = mod_syntax("type foo = bar;\nenum bar { a }").expect("valid");
        let m = pkg.module("mod").expect("get module");
        let foo = m.datatype("foo").expect("get foo");

        match foo.variant() {
            DatatypeVariant::Alias(a) => {
                assert_eq!(a.datatype().name(), "foo");
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

    #[test]
    fn func_trivial() {
        let pkg = mod_syntax("fn foo();").expect("valid module");
        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");
        assert_eq!(foo.name(), "foo");
        assert_eq!(foo.params().collect::<Vec<_>>().len(), 0);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 0);
    }

    #[test]
    fn func_one_arg() {
        let pkg = mod_syntax("fn foo(a: i64);").expect("valid module");
        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");

        assert_eq!(foo.args().collect::<Vec<_>>().len(), 1);
        let a = foo.arg("a").expect("arg a exists");
        assert_eq!(a.name(), "a");
        assert_eq!(a.type_().name(), "i64");
        assert_eq!(a.binding().name(), "a");

        assert_eq!(foo.rets().collect::<Vec<_>>().len(), 0);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 1);

        let a_bind = foo.binding("a").expect("binding a exists");
        assert_eq!(a_bind.name(), "a");
        assert_eq!(a_bind.type_().name(), "i64");
        assert_eq!(a_bind.direction(), BindingDirection::In);
        let a_val = a_bind.param().value().expect("binding is a value");
        assert_eq!(a_val.name(), "a");
        assert_eq!(a_val.param_position(), ParamPosition::Arg(0));
    }

    #[test]
    fn func_one_ret() {
        let pkg = mod_syntax("fn foo() -> r: i64;").expect("valid module");
        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");

        assert_eq!(foo.rets().collect::<Vec<_>>().len(), 1);
        let r = foo.ret("r").expect("ret r exists");
        assert_eq!(r.name(), "r");
        assert_eq!(r.type_().name(), "i64");
        assert_eq!(r.binding().name(), "r");

        assert_eq!(foo.args().collect::<Vec<_>>().len(), 0);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 1);

        let r_bind = foo.binding("r").expect("binding r exists");
        assert_eq!(r_bind.name(), "r");
        assert_eq!(r_bind.type_().name(), "i64");
        assert_eq!(r_bind.direction(), BindingDirection::Out);
        let r_val = r_bind.param().value().expect("binding is a value");
        assert_eq!(r_val.name(), "r");
        assert_eq!(r_val.param_position(), ParamPosition::Ret(0));
    }

    #[test]
    fn func_multiple_returns() {
        assert_eq!(
            mod_syntax("fn trivial(a: i32) -> r1: i32, r2: i32;")
                .err()
                .unwrap(),
            ValidationError::Syntax {
                expected: "at most one return value",
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn func_duplicate_arg() {
        assert_eq!(
            mod_syntax("fn trivial(a: i32, a: i32);").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location {
                    line: 1,
                    column: 19
                },
                previous_location: Location {
                    line: 1,
                    column: 11
                },
            }
        );
    }

    #[test]
    fn func_one_arg_value_binding() {
        let pkg = mod_syntax("fn foo(a: i32) where\na_binding: in i8 <- a;").expect("valid");
        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");

        assert_eq!(foo.args().collect::<Vec<_>>().len(), 1);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 1);

        let a = foo.arg("a").expect("arg a exists");
        assert_eq!(a.name(), "a");
        assert_eq!(a.type_().name(), "i32");
        assert_eq!(a.binding().direction(), BindingDirection::In);
        assert_eq!(a.binding().name(), "a_binding");
        a.binding()
            .param()
            .value()
            .expect("binding param used as value");
        assert_eq!(a.binding().type_().name(), "i8");
    }

    #[test]
    fn func_one_arg_ptr_binding() {
        let pkg = mod_syntax("fn foo(a: i32) where\na_binding: inout i8 <- *a;").expect("valid");
        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");

        assert_eq!(foo.args().collect::<Vec<_>>().len(), 1);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 1);

        let a = foo.arg("a").expect("arg a exists");
        assert_eq!(a.name(), "a");
        assert_eq!(a.type_().name(), "i32");
        assert_eq!(a.binding().direction(), BindingDirection::InOut);
        assert_eq!(a.binding().name(), "a_binding");
        a.binding()
            .param()
            .ptr()
            .expect("binding param used as ptr");
        assert_eq!(a.binding().type_().name(), "i8");
    }

    #[test]
    fn func_one_arg_binding_wrong_direction() {
        assert_eq!(
            mod_syntax("fn foo(a: i32) where\na_binding: out i8 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "argument value must be input-only binding",
                location: Location { line: 2, column: 0 }
            },
        );
    }

    #[test]
    fn func_one_arg_binding_wrong_type() {
        // Cant convert int to float
        assert_eq!(
            mod_syntax("fn trivial(a: i32) where\na_binding: in f32 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type which can represent argument type",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant convert float to int
        assert_eq!(
            mod_syntax("fn trivial(a: f32) where\na_binding: in i32 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type which can represent argument type",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant represent i64 with i32
        assert_eq!(
            mod_syntax("fn trivial(a: i32) where\na_binding: in i64 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type which can represent argument type",
                location: Location { line: 2, column: 0 }
            },
        );

        // but, can represent i32 with i64
        mod_syntax("fn trivial(a: i64) where\na_binding: in i32 <- a;").unwrap();

        // Cant represent f64 with f32
        assert_eq!(
            mod_syntax("fn trivial(a: f32) where\na_binding: in f64 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type which can represent argument type",
                location: Location { line: 2, column: 0 }
            },
        );

        // Cant represent ptr with f32
        assert_eq!(
            mod_syntax("fn trivial(a: f32) where\na_binding: in i8 <- *a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "pointer bindings to be represented as an i32",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant represent ptr with i64
        assert_eq!(
            mod_syntax("fn trivial(a: i64) where\na_binding: out i8 <- *a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "pointer bindings to be represented as an i32",
                location: Location { line: 2, column: 0 }
            },
        );
    }

    #[test]
    fn func_one_ret_value_binding() {
        let pkg = mod_syntax("fn foo() -> a: i32 where\na_binding: out i8 <- a;").expect("valid");

        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");

        assert_eq!(foo.rets().collect::<Vec<_>>().len(), 1);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 1);

        let a = foo.ret("a").expect("ret a exists");
        assert_eq!(a.name(), "a");
        assert_eq!(a.type_().name(), "i32");
        assert_eq!(a.binding().direction(), BindingDirection::Out);
        assert_eq!(a.binding().name(), "a_binding");
        a.binding()
            .param()
            .value()
            .expect("binding param used as value");
        assert_eq!(a.binding().type_().name(), "i8");
    }

    #[test]
    fn func_one_ret_pointer_binding() {
        assert_eq!(
            mod_syntax("fn trivial() -> a: i32 where\na_binding: out i8 <- *a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "return value cannot be bound to pointer",
                location: Location { line: 2, column: 0 },
            }
        );
    }

    #[test]
    fn func_one_ret_wrong_direction() {
        assert_eq!(
            mod_syntax("fn trivial() -> a: i32 where\na_binding: in i8 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "return value must be output-only binding",
                location: Location { line: 2, column: 0 }
            },
        );
    }

    #[test]
    fn func_two_arg_slice_binding() {
        let pkg = mod_syntax(
            "fn foo(a_ptr: i32, a_len: i32) where\na_binding: inout i8 <- [a_ptr, a_len];",
        )
        .expect("valid");
        let m = pkg.module("mod").expect("get module");
        let foo = m.function("foo").expect("get foo");

        assert_eq!(foo.args().collect::<Vec<_>>().len(), 2);
        assert_eq!(foo.bindings().collect::<Vec<_>>().len(), 1);

        let a_ptr = foo.arg("a_ptr").expect("arg a_ptr exists");
        let a_len = foo.arg("a_len").expect("arg a_len exists");
        assert_eq!(a_ptr.type_().name(), "i32");
        assert_eq!(a_len.type_().name(), "i32");

        assert_eq!(a_ptr.binding().name(), "a_binding");
        assert_eq!(a_len.binding().name(), "a_binding");
        assert_eq!(a_ptr.binding().type_().name(), "i8");
        let (a_ptr_2, a_len_2) = a_ptr
            .binding()
            .param()
            .slice()
            .expect("binding param used as slice");
        assert_eq!(a_ptr_2.name(), "a_ptr");
        assert_eq!(a_len_2.name(), "a_len");
    }

    #[test]
    fn func_buncha_bindings() {
        let pkg = mod_syntax(
            "fn nontrivial(a: i32, b: i32, c: f32) -> d: i32 where\n\
             a_binding: out u8 <- *a,\n\
             b_binding: inout u16 <- *b,\n\
             c_binding: in f32 <- c,\n\
             d_binding: out i8 <- d;\n\
             ",
        )
        .expect("valid");

        let m = pkg.module("mod").expect("get module");
        let nontrivial = m.function("nontrivial").expect("get nontrivial");

        assert_eq!(nontrivial.args().collect::<Vec<_>>().len(), 3);
        assert_eq!(nontrivial.rets().collect::<Vec<_>>().len(), 1);
        assert_eq!(nontrivial.bindings().collect::<Vec<_>>().len(), 4);

        let a = nontrivial.arg("a").expect("arg a exists");
        assert_eq!(a.type_().name(), "i32");
        let b = nontrivial.arg("b").expect("arg b exists");
        assert_eq!(b.type_().name(), "i32");
        let c = nontrivial.arg("c").expect("arg c exists");
        assert_eq!(c.type_().name(), "f32");
        let d = nontrivial.ret("d").expect("ret d exists");
        assert_eq!(d.type_().name(), "i32");

        let a_binding = nontrivial.binding("a_binding").expect("a_binding exists");
        assert_eq!(a_binding.direction(), BindingDirection::Out);
        let b_binding = nontrivial.binding("b_binding").expect("b_binding exists");
        assert_eq!(b_binding.direction(), BindingDirection::InOut);
        let c_binding = nontrivial.binding("c_binding").expect("c_binding exists");
        assert_eq!(c_binding.direction(), BindingDirection::In);
        let d_binding = nontrivial.binding("d_binding").expect("d_binding exists");
        assert_eq!(d_binding.direction(), BindingDirection::Out);
    }
}
