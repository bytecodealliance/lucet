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
