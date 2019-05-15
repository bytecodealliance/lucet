use crate::data_layout::{
    AliasIR, DataTypeModuleBuilder, EnumIR, StructIR, StructMemberIR, VariantIR,
};
use crate::error::ValidationError;
use crate::parser::{SyntaxDecl, SyntaxRef};
use crate::types::{
    Attr, DataType, DataTypeRef, EnumMember, FuncArg, FuncDecl, FuncRet, Ident, Location, Name,
    Named,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Module {
    pub names: Vec<Name>,
    pub attrs: Vec<Attr>,
    pub data_types: HashMap<Ident, DataType>,
    pub data_type_ordering: Vec<Ident>,
    pub funcs: HashMap<Ident, FuncDecl>,
}

impl Module {
    fn new(attrs: &[Attr]) -> Self {
        Self {
            names: Vec::new(),
            attrs: attrs.to_vec(),
            data_types: HashMap::new(),
            data_type_ordering: Vec::new(),
            funcs: HashMap::new(),
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

    fn get_ref(&self, syntax_ref: &SyntaxRef) -> Result<DataTypeRef, ValidationError> {
        match syntax_ref {
            SyntaxRef::Atom { atom, .. } => Ok(DataTypeRef::Atom(*atom)),
            SyntaxRef::Name { name, location } => match self.id_for_name(name) {
                Some(id) => Ok(DataTypeRef::Defined(id)),
                None => Err(ValidationError::NameNotFound {
                    name: name.clone(),
                    use_location: *location,
                }),
            },
        }
    }

    fn decl_to_ir(
        &self,
        id: Ident,
        decl: &SyntaxDecl,
        data_types_ir: &mut DataTypeModuleBuilder,
        funcs_ir: &mut HashMap<Ident, FuncDecl>,
    ) -> Result<(), ValidationError> {
        match decl {
            SyntaxDecl::Struct {
                name,
                members,
                attrs,
                location,
            } => {
                let mut uniq_membs = HashMap::new();
                let mut dtype_members = Vec::new();
                if members.is_empty() {
                    Err(ValidationError::Empty {
                        name: name.clone(),
                        location: *location,
                    })?
                }
                for mem in members {
                    // Ensure that each member name is unique:
                    if let Some(existing) = uniq_membs.insert(mem.name.clone(), mem) {
                        Err(ValidationError::NameAlreadyExists {
                            name: mem.name.clone(),
                            at_location: mem.location,
                            previous_location: existing.location,
                        })?
                    }
                    // Get the DataTypeRef for the member, which ensures that it refers only to
                    // defined types:
                    let type_ = self.get_ref(&mem.type_)?;
                    // build the struct with this as the member:
                    dtype_members.push(StructMemberIR {
                        type_,
                        name: mem.name.clone(),
                        attrs: mem.attrs.clone(),
                    })
                }

                data_types_ir.define(
                    id,
                    VariantIR::Struct(StructIR {
                        members: dtype_members,
                    }),
                    attrs.clone(),
                    location.clone(),
                );
            }
            SyntaxDecl::Enum {
                name,
                variants,
                attrs,
                location,
            } => {
                let mut uniq_vars = HashMap::new();
                let mut dtype_members = Vec::new();
                if variants.is_empty() {
                    Err(ValidationError::Empty {
                        name: name.clone(),
                        location: *location,
                    })?
                }
                for var in variants {
                    // Ensure that each member name is unique:
                    if let Some(existing) = uniq_vars.insert(var.name.clone(), var) {
                        Err(ValidationError::NameAlreadyExists {
                            name: var.name.clone(),
                            at_location: var.location,
                            previous_location: existing.location,
                        })?
                    }
                    // build the struct with this as the member:
                    dtype_members.push(EnumMember {
                        name: var.name.clone(),
                        attrs: var.attrs.clone(),
                    })
                }
                data_types_ir.define(
                    id,
                    VariantIR::Enum(EnumIR {
                        members: dtype_members,
                    }),
                    attrs.clone(),
                    location.clone(),
                );
            }
            SyntaxDecl::Alias {
                what,
                attrs,
                location,
                ..
            } => {
                let to = self.get_ref(what)?;
                data_types_ir.define(
                    id,
                    VariantIR::Alias(AliasIR { to }),
                    attrs.clone(),
                    location.clone(),
                );
            }
            SyntaxDecl::Function {
                args,
                rets,
                attrs,
                location,
                ..
            } => {
                let mut arg_names: HashMap<String, Location> = HashMap::new();
                let args = args
                    .iter()
                    .map(|arg_syntax| {
                        let type_ = self.get_ref(&arg_syntax.type_)?;
                        if let Some(previous_location) = arg_names.get(&arg_syntax.name) {
                            Err(ValidationError::NameAlreadyExists {
                                name: arg_syntax.name.clone(),
                                at_location: arg_syntax.location,
                                previous_location: previous_location.clone(),
                            })?;
                        } else {
                            arg_names.insert(arg_syntax.name.clone(), arg_syntax.location.clone());
                        }
                        Ok(FuncArg {
                            name: arg_syntax.name.clone(),
                            type_,
                            attrs: arg_syntax.attrs.clone(),
                        })
                    })
                    .collect::<Result<Vec<FuncArg>, _>>()?;

                let rets = rets
                    .iter()
                    .map(|ret_syntax| {
                        let type_ = self.get_ref(&ret_syntax.type_)?;
                        Ok(FuncRet {
                            type_,
                            attrs: ret_syntax.attrs.clone(),
                        })
                    })
                    .collect::<Result<Vec<FuncRet>, _>>()?;
                if rets.len() > 1 {
                    Err(ValidationError::Syntax {
                        expected: "at most one return value",
                        location: location.clone(),
                    })?
                }

                let decl = FuncDecl {
                    args,
                    rets,
                    attrs: attrs.clone(),
                };
                if let Some(prev_def) = funcs_ir.insert(id, decl) {
                    panic!("id {} already defined: {:?}", id, prev_def)
                }
            }
            SyntaxDecl::Module { .. } => unreachable!(), // Should be excluded by from_declarations constructor
        }
        Ok(())
    }

    pub fn from_declarations(
        decls: &[SyntaxDecl],
        attrs: &[Attr],
    ) -> Result<Module, ValidationError> {
        let mut mod_ = Self::new(attrs);
        let mut idents: Vec<Ident> = Vec::new();
        for decl in decls {
            match decl {
                SyntaxDecl::Module { .. } => Err(ValidationError::Syntax {
                    expected: "type or function declaration",
                    location: *decl.location(),
                })?,
                _ => idents.push(mod_.introduce_name(decl.name(), decl.location())?),
            }
        }

        let mut data_types_ir = DataTypeModuleBuilder::new();
        let mut funcs_ir = HashMap::new();
        for (decl, id) in decls.iter().zip(&idents) {
            mod_.decl_to_ir(id.clone(), decl, &mut data_types_ir, &mut funcs_ir)?
        }

        let (data_types, ordering) = data_types_ir.validate_datatypes(&mod_.names)?;
        mod_.data_types = data_types;
        mod_.data_type_ordering = ordering;

        mod_.funcs = funcs_ir;

        Ok(mod_)
    }

    /// Retrieve information about a data type given its identifier
    pub fn get_datatype(&self, id: Ident) -> Option<Named<DataType>> {
        let name = &self.names[id.0];
        if let Some(data_type) = &self.data_types.get(&id) {
            Some(Named {
                id,
                name,
                entity: data_type,
            })
        } else {
            None
        }
    }

    /// Retrieve information about a function declaration  given its identifier
    pub fn get_func_decl(&self, id: Ident) -> Option<Named<FuncDecl>> {
        let name = &self.names[id.0];
        if let Some(func_decl) = &self.funcs.get(&id) {
            Some(Named {
                id,
                name,
                entity: func_decl,
            })
        } else {
            None
        }
    }
    pub fn datatypes(&self) -> impl Iterator<Item = Named<DataType>> {
        self.data_type_ordering
            .iter()
            .map(move |i| self.get_datatype(*i).unwrap())
    }

    pub fn func_decls(&self) -> impl Iterator<Item = Named<FuncDecl>> {
        self.funcs
            .iter()
            .map(move |(i, _)| self.get_func_decl(*i).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::types::{AliasDataType, AtomType, DataTypeVariant, StructDataType, StructMember};

    fn mod_(syntax: &str) -> Result<Module, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_decls().expect("parses");
        Module::from_declarations(&decls, &[])
    }

    #[test]
    fn structs_basic() {
        assert!(mod_("struct foo { a: i32}").is_ok());
        assert!(mod_("struct foo { a: i32, b: f32 }").is_ok());
    }

    #[test]
    fn struct_two_atoms() {
        {
            let d = mod_("struct foo { a: i32, b: f32 }").unwrap();
            assert_eq!(
                d.data_types[&Ident(0)],
                DataType {
                    variant: DataTypeVariant::Struct(StructDataType {
                        members: vec![
                            StructMember {
                                name: "a".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::I32),
                                attrs: Vec::new(),
                                offset: 0,
                            },
                            StructMember {
                                name: "b".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::F32),
                                attrs: Vec::new(),
                                offset: 4,
                            },
                        ]
                    }),
                    attrs: Vec::new(),
                    repr_size: 8,
                    align: 4,
                }
            );
        }
    }

    #[test]
    fn struct_prev_definition() {
        // Refer to a struct defined previously:
        assert!(mod_("struct foo { a: i32, b: f64 } struct bar { a: foo }").is_ok());
    }

    #[test]
    fn struct_next_definition() {
        // Refer to a struct defined afterwards:
        assert!(mod_("struct foo { a: i32, b: bar} struct bar { a: i32 }").is_ok());
    }

    #[test]
    fn struct_self_referential() {
        // Refer to itself
        assert!(mod_("struct list { next: list, thing: i32 }").is_err());
    }

    #[test]
    fn struct_empty() {
        // No members
        assert_eq!(
            mod_("struct foo {}").err().unwrap(),
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
            mod_("struct foo { \na: i32, \na: f64}").err().unwrap(),
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
            mod_("struct foo { a: i32 }\nstruct foo { a: i32 } ")
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
            mod_("struct foo { \nb: bar }").err().unwrap(),
            ValidationError::NameNotFound {
                name: "bar".to_owned(),
                use_location: Location { line: 2, column: 3 },
            }
        );
    }

    #[test]
    fn enums() {
        assert!(mod_("enum foo { a }").is_ok());
        assert!(mod_("enum foo { a, b }").is_ok());

        {
            let d = mod_("enum foo { a, b }").unwrap();
            let members = match &d.data_types[&Ident(0)].variant {
                DataTypeVariant::Enum(e) => &e.members,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(members[0].name, "a");
            assert_eq!(members[1].name, "b");
        }

        // No members
        assert_eq!(
            mod_("enum foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        // Duplicate member in enum
        assert_eq!(
            mod_("enum foo { \na,\na }").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );

        // Duplicate definition of enum
        assert_eq!(
            mod_("enum foo { a }\nenum foo { a } ").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn aliases() {
        assert!(mod_("type foo = i32;").is_ok());
        assert!(mod_("type foo = f64;").is_ok());
        assert!(mod_("type foo = u8;").is_ok());

        assert!(mod_("type foo = bar;\nenum bar { a }").is_ok());

        assert!(mod_("type link = u32;\nstruct list { next: link, thing: i32 }").is_ok());
    }

    #[test]
    fn infinite() {
        assert_eq!(
            mod_("type foo = bar;\ntype bar = foo;").err().unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            mod_("type foo = bar;\nstruct bar { a: foo }")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            mod_("type foo = bar;\nstruct bar { a: baz }\nstruct baz { c: i32, e: foo }")
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
        assert_eq!(
            mod_("fn trivial();").ok().unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        args: Vec::new(),
                        rets: Vec::new(),
                        attrs: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                attrs: Vec::new(),
            }
        );
    }
    #[test]
    fn func_one_arg() {
        assert_eq!(
            mod_("fn trivial(a: u8);").ok().unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        args: vec![FuncArg {
                            type_: DataTypeRef::Atom(AtomType::U8),
                            name: "a".to_owned(),
                            attrs: Vec::new(),
                        }],
                        rets: Vec::new(),
                        attrs: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                attrs: Vec::new(),
            }
        );
    }

    #[test]
    fn func_one_ret() {
        assert_eq!(
            mod_("fn trivial() -> u8;").ok().unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        args: Vec::new(),
                        rets: vec![FuncRet {
                            type_: DataTypeRef::Atom(AtomType::U8),
                            attrs: Vec::new(),
                        }],
                        attrs: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                attrs: Vec::new(),
            }
        );
    }

    #[test]
    fn func_one_ret_defined_type() {
        assert_eq!(
            mod_("fn trivial() -> foo;\ntype foo = u8;").ok().unwrap(),
            Module {
                names: vec![
                    Name {
                        name: "trivial".to_owned(),
                        location: Location { line: 1, column: 0 }
                    },
                    Name {
                        name: "foo".to_owned(),
                        location: Location { line: 2, column: 0 }
                    }
                ],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        args: Vec::new(),
                        rets: vec![FuncRet {
                            type_: DataTypeRef::Defined(Ident(1)),
                            attrs: Vec::new(),
                        }],
                        attrs: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: vec![(
                    Ident(1),
                    DataType {
                        variant: DataTypeVariant::Alias(AliasDataType {
                            to: DataTypeRef::Atom(AtomType::U8),
                        }),
                        attrs: Vec::new(),
                        repr_size: 1,
                        align: 1,
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_type_ordering: vec![Ident(1)],
                attrs: Vec::new(),
            }
        );
    }

    #[test]
    fn func_unknown_arg_type() {
        assert_eq!(
            mod_("fn trivial(a: foo);").err().unwrap(),
            ValidationError::NameNotFound {
                name: "foo".to_owned(),
                use_location: Location {
                    line: 1,
                    column: 14
                },
            }
        );
    }

    #[test]
    fn func_unknown_ret_type() {
        assert_eq!(
            mod_("fn trivial(a: u8) -> foo;").err().unwrap(),
            ValidationError::NameNotFound {
                name: "foo".to_owned(),
                use_location: Location {
                    line: 1,
                    column: 21
                },
            }
        );
    }

    #[test]
    fn func_multiple_returns() {
        assert_eq!(
            mod_("fn trivial(a: u8) -> bool, bool;").err().unwrap(),
            ValidationError::Syntax {
                expected: "at most one return value",
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn func_duplicate_arg() {
        assert_eq!(
            mod_("fn trivial(a: u8, a: u8);").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location {
                    line: 1,
                    column: 18
                },
                previous_location: Location {
                    line: 1,
                    column: 11
                },
            }
        );
    }
}
