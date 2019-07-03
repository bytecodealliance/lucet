use crate::data_layout::{
    AliasIR, DataTypeModuleBuilder, EnumIR, StructIR, StructMemberIR, VariantIR,
};
use crate::error::ValidationError;
use crate::function::validate_func_args;
use crate::parser::{SyntaxDecl, SyntaxTypeRef};
use crate::types::{
    AbiType, DataType, DataTypeRef, DataTypeVariant, EnumMember, FuncDecl, Ident, Location, Name,
    Named,
};
use heck::SnakeCase;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Module {
    pub names: Vec<Name>,
    pub data_types: HashMap<Ident, DataType>,
    pub data_type_ordering: Vec<Ident>,
    pub funcs: HashMap<Ident, FuncDecl>,
    pub module_name: String,
    pub binding_prefix: String,
}

impl Module {
    fn new(module_name: String, binding_prefix: String) -> Self {
        Self {
            names: Vec::new(),
            data_types: HashMap::new(),
            data_type_ordering: Vec::new(),
            funcs: HashMap::new(),
            module_name,
            binding_prefix,
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

    pub fn get_typeref(&self, syntax_ref: &SyntaxTypeRef) -> Result<DataTypeRef, ValidationError> {
        match syntax_ref {
            SyntaxTypeRef::Atom { atom, .. } => Ok(DataTypeRef::Atom(*atom)),
            SyntaxTypeRef::Name { name, location } => match self.id_for_name(name) {
                Some(id) => Ok(DataTypeRef::Defined(id)),
                None => Err(ValidationError::NameNotFound {
                    name: name.clone(),
                    use_location: *location,
                }),
            },
        }
    }

    // If a DataType is representable by an atomic AbiType, get it. Otherwise the DataType must
    // be pointed to across ABI boundaries
    pub fn get_abi_repr(&self, dtref: &DataTypeRef) -> Option<AbiType> {
        match dtref {
            DataTypeRef::Defined(ident) => {
                let dt = self
                    .data_types
                    .get(ident)
                    .expect("abi representation of valid dtref");
                match &dt.variant {
                    DataTypeVariant::Struct(struct_) => {
                        if struct_.members.len() == 1 {
                            self.get_abi_repr(&struct_.members[0].type_)
                        } else {
                            None
                        }
                    }
                    DataTypeVariant::Enum(enum_) => {
                        if enum_.members.len() < u32::max_value() as usize {
                            Some(AbiType::I32)
                        } else {
                            None
                        }
                    }
                    DataTypeVariant::Alias(alias) => self.get_abi_repr(&alias.to),
                }
            }
            DataTypeRef::Atom(atom) => Some(AbiType::from_atom(atom)),
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
                    let type_ = self.get_typeref(&mem.type_)?;
                    // build the struct with this as the member:
                    dtype_members.push(StructMemberIR {
                        type_,
                        name: mem.name.clone(),
                    })
                }

                data_types_ir.define(
                    id,
                    VariantIR::Struct(StructIR {
                        members: dtype_members,
                    }),
                    location.clone(),
                );
            }
            SyntaxDecl::Enum {
                name,
                variants,
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
                    })
                }
                data_types_ir.define(
                    id,
                    VariantIR::Enum(EnumIR {
                        members: dtype_members,
                    }),
                    location.clone(),
                );
            }
            SyntaxDecl::Alias { what, location, .. } => {
                let to = self.get_typeref(what)?;
                data_types_ir.define(id, VariantIR::Alias(AliasIR { to }), location.clone());
            }
            SyntaxDecl::Function {
                name,
                args,
                rets,
                location,
                bindings,
            } => {
                let (args, rets, bindings) =
                    validate_func_args(args, rets, bindings, location, self)?;
                let binding_name = self.binding_prefix.clone() + "_" + &name.to_snake_case();
                let decl = FuncDecl {
                    field_name: name.clone(),
                    binding_name,
                    args,
                    rets,
                    bindings,
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
        module_name: String,
        binding_prefix: String,
    ) -> Result<Module, ValidationError> {
        let mut mod_ = Self::new(module_name, binding_prefix);
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

    pub fn func_bindings(&self) -> HashMap<String, String> {
        self.func_decls()
            .map(|d| (d.entity.field_name.clone(), d.entity.binding_name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::types::{
        AbiType, AtomType, BindDirection, BindingRef, DataTypeVariant, FuncArg, FuncBinding,
        StructDataType, StructMember,
    };

    fn mod_(syntax: &str) -> Result<Module, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_decls().expect("parses");
        Module::from_declarations(&decls, String::new(), String::new())
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
                                offset: 0,
                            },
                            StructMember {
                                name: "b".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::F32),
                                offset: 4,
                            },
                        ]
                    }),
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
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: Vec::new(),
                        rets: Vec::new(),
                        bindings: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }
    #[test]
    fn func_one_arg() {
        assert_eq!(
            mod_("fn trivial(a: i64);").ok().unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: vec![FuncArg {
                            type_: AbiType::I64,
                            name: "a".to_owned(),
                        }],
                        rets: Vec::new(),
                        bindings: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }

    #[test]
    fn func_one_ret() {
        assert_eq!(
            mod_("fn trivial() -> r: i64;").ok().unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: Vec::new(),
                        rets: vec![FuncArg {
                            name: "r".to_owned(),
                            type_: AbiType::I64,
                        }],
                        bindings: Vec::new(),
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }

    #[test]
    fn func_multiple_returns() {
        assert_eq!(
            mod_("fn trivial(a: i32) -> r1: i32, r2: i32;")
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
            mod_("fn trivial(a: i32, a: i32);").err().unwrap(),
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
        assert_eq!(
            mod_("fn trivial(a: i32) where\na_binding: in i8 <- a;")
                .ok()
                .unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: vec![FuncArg {
                            type_: AbiType::I32,
                            name: "a".to_owned(),
                        }],
                        rets: Vec::new(),
                        bindings: vec![FuncBinding {
                            name: "a_binding".to_owned(),
                            type_: DataTypeRef::Atom(AtomType::I8),
                            direction: BindDirection::In,
                            from: BindingRef::Value("a".to_owned()),
                        }],
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }

    #[test]
    fn func_one_arg_ptr_binding() {
        assert_eq!(
            mod_("fn trivial(a: i32) where\na_binding: inout i8 <- *a;")
                .ok()
                .unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: vec![FuncArg {
                            type_: AbiType::I32,
                            name: "a".to_owned(),
                        }],
                        rets: Vec::new(),
                        bindings: vec![FuncBinding {
                            name: "a_binding".to_owned(),
                            type_: DataTypeRef::Atom(AtomType::I8),
                            direction: BindDirection::InOut,
                            from: BindingRef::Ptr("a".to_owned()),
                        }],
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }

    #[test]
    fn func_one_arg_binding_wrong_direction() {
        assert_eq!(
            mod_("fn trivial(a: i32) where\na_binding: out i8 <- a;")
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
            mod_("fn trivial(a: i32) where\na_binding: out f32 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type representation to match argument type",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant convert float to int
        assert_eq!(
            mod_("fn trivial(a: f32) where\na_binding: out i32 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type representation to match argument type",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant represent i64 with i32
        assert_eq!(
            mod_("fn trivial(a: i32) where\na_binding: out i64 <- a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "binding type representation to match argument type",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant represent ptr with float
        assert_eq!(
            mod_("fn trivial(a: f32) where\na_binding: out i8 <- *a;")
                .err()
                .unwrap(),
            ValidationError::BindingTypeError {
                expected: "pointer bindings to be represented as an i32",
                location: Location { line: 2, column: 0 }
            },
        );
        // Cant represent ptr with i64
        assert_eq!(
            mod_("fn trivial(a: i64) where\na_binding: out i8 <- *a;")
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
        assert_eq!(
            mod_("fn trivial() -> a: i32 where\na_binding: out i8 <- a;")
                .ok()
                .unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: Vec::new(),
                        rets: vec![FuncArg {
                            type_: AbiType::I32,
                            name: "a".to_owned(),
                        }],
                        bindings: vec![FuncBinding {
                            name: "a_binding".to_owned(),
                            type_: DataTypeRef::Atom(AtomType::I8),
                            direction: BindDirection::Out,
                            from: BindingRef::Value("a".to_owned()),
                        }],
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }

    #[test]
    fn func_one_ret_pointer_binding() {
        assert_eq!(
            mod_("fn trivial() -> a: i32 where\na_binding: out i8 <- *a;")
                .ok()
                .unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: Vec::new(),
                        rets: vec![FuncArg {
                            type_: AbiType::I32,
                            name: "a".to_owned(),
                        }],
                        bindings: vec![FuncBinding {
                            name: "a_binding".to_owned(),
                            type_: DataTypeRef::Atom(AtomType::I8),
                            direction: BindDirection::Out,
                            from: BindingRef::Ptr("a".to_owned()),
                        }],
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }
    #[test]
    fn func_one_ret_wrong_direction() {
        assert_eq!(
            mod_("fn trivial() -> a: i32 where\na_binding: in i8 <- a;")
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
        assert_eq!(
            mod_(
                "fn trivial(a_ptr: i32, a_len: i32) where\na_binding: inout i8 <- [a_ptr, a_len];"
            )
            .ok()
            .unwrap(),
            Module {
                names: vec![Name {
                    name: "trivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_trivial".to_owned(),
                        field_name: "trivial".to_owned(),
                        args: vec![
                            FuncArg {
                                type_: AbiType::I32,
                                name: "a_ptr".to_owned(),
                            },
                            FuncArg {
                                type_: AbiType::I32,
                                name: "a_len".to_owned(),
                            }
                        ],
                        rets: Vec::new(),
                        bindings: vec![FuncBinding {
                            name: "a_binding".to_owned(),
                            type_: DataTypeRef::Atom(AtomType::I8),
                            direction: BindDirection::InOut,
                            from: BindingRef::Slice("a_ptr".to_owned(), "a_len".to_owned()),
                        }],
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }

    #[test]
    fn func_buncha_bindings() {
        assert_eq!(
            mod_(
                "fn nontrivial(a: i32, b: i32, c: f32) -> d: i32 where\n\
                 a_binding: out u8 <- *a,\n\
                 b_binding: inout u16 <- *b,\n\
                 c_binding: in f32 <- c,\n\
                 d_binding: out i8 <- *d;\n\
                 "
            )
            .ok()
            .unwrap(),
            Module {
                names: vec![Name {
                    name: "nontrivial".to_owned(),
                    location: Location { line: 1, column: 0 }
                }],
                funcs: vec![(
                    Ident(0),
                    FuncDecl {
                        binding_name: "_nontrivial".to_owned(),
                        field_name: "nontrivial".to_owned(),
                        args: vec![
                            FuncArg {
                                type_: AbiType::I32,
                                name: "a".to_owned(),
                            },
                            FuncArg {
                                type_: AbiType::I32,
                                name: "b".to_owned(),
                            },
                            FuncArg {
                                type_: AbiType::F32,
                                name: "c".to_owned(),
                            }
                        ],
                        rets: vec![FuncArg {
                            type_: AbiType::I32,
                            name: "d".to_owned(),
                        }],
                        bindings: vec![
                            FuncBinding {
                                name: "a_binding".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::U8),
                                direction: BindDirection::Out,
                                from: BindingRef::Ptr("a".to_owned()),
                            },
                            FuncBinding {
                                name: "b_binding".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::U16),
                                direction: BindDirection::InOut,
                                from: BindingRef::Ptr("b".to_owned()),
                            },
                            FuncBinding {
                                name: "c_binding".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::F32),
                                direction: BindDirection::In,
                                from: BindingRef::Value("c".to_owned()),
                            },
                            FuncBinding {
                                name: "d_binding".to_owned(),
                                type_: DataTypeRef::Atom(AtomType::I8),
                                direction: BindDirection::Out,
                                from: BindingRef::Ptr("d".to_owned()),
                            },
                        ],
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
                data_types: HashMap::new(),
                data_type_ordering: Vec::new(),
                module_name: String::new(),
                binding_prefix: String::new(),
            }
        );
    }
}
