use crate::error::ValidationError;
use crate::parser::{SyntaxDecl, SyntaxRef};
use crate::types::{
    Attr, DataType, DataTypeRef, FuncDecl, FuncRet, Ident, Location, Name, Named, NamedMember,
};
use std::collections::HashMap;
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Module {
    pub names: Vec<Name>,
    pub attrs: Vec<Attr>,
    pub data_types: HashMap<Ident, DataType>,
    pub funcs: HashMap<Ident, FuncDecl>,
}

impl Module {
    fn new(attrs: &[Attr]) -> Self {
        Self {
            names: Vec::new(),
            attrs: attrs.to_vec(),
            data_types: HashMap::new(),
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

    fn define_data_type(&mut self, id: Ident, dt: DataType) {
        if let Some(prev_def) = self.data_types.insert(id, dt) {
            panic!("id {} already defined: {:?}", id, prev_def)
        }
    }

    fn define_function(&mut self, id: Ident, decl: FuncDecl) {
        if let Some(prev_def) = self.funcs.insert(id, decl) {
            panic!("id {} already defined: {:?}", id, prev_def)
        }
    }

    fn define_decl(&mut self, id: Ident, decl: &SyntaxDecl) -> Result<(), ValidationError> {
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
                    dtype_members.push(NamedMember {
                        type_,
                        name: mem.name.clone(),
                        attrs: mem.attrs.clone(),
                    })
                }
                self.define_data_type(
                    id,
                    DataType::Struct {
                        members: dtype_members,
                        attrs: attrs.clone(),
                    },
                )
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
                    dtype_members.push(NamedMember {
                        type_: (),
                        name: var.name.clone(),
                        attrs: var.attrs.clone(),
                    })
                }
                self.define_data_type(
                    id,
                    DataType::Enum {
                        members: dtype_members,
                        attrs: attrs.clone(),
                    },
                )
            }
            SyntaxDecl::Alias { what, attrs, .. } => {
                let to = self.get_ref(what)?;
                self.define_data_type(
                    id,
                    DataType::Alias {
                        to,
                        attrs: attrs.clone(),
                    },
                );
            }
            SyntaxDecl::Function {
                args, rets, attrs, ..
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
                        Ok(NamedMember {
                            name: arg_syntax.name.clone(),
                            type_,
                            attrs: arg_syntax.attrs.clone(),
                        })
                    })
                    .collect::<Result<Vec<NamedMember<DataTypeRef>>, _>>()?;

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

                self.define_function(
                    id,
                    FuncDecl {
                        args,
                        rets,
                        attrs: attrs.clone(),
                    },
                );
            }
            SyntaxDecl::Module { .. } => unreachable!(), // Should be excluded by from_declarations constructor
        }
        Ok(())
    }

    fn dfs_walk(
        &self,
        id: Ident,
        visited: &mut [bool],
        ordered: &mut Option<&mut Vec<Ident>>,
    ) -> Result<(), ()> {
        if visited[id.0] {
            Err(())?
        }
        visited[id.0] = true;
        match self.data_types.get(&id).expect("data_type is defined") {
            DataType::Struct { members, .. } => {
                for mem in members {
                    if let DataTypeRef::Defined(id) = mem.type_ {
                        self.dfs_walk(id, visited, ordered)?
                    }
                }
            }
            DataType::Alias { to, .. } => {
                if let DataTypeRef::Defined(id) = to {
                    self.dfs_walk(*id, visited, ordered)?
                }
            }
            DataType::Enum { .. } => {}
        }
        if let Some(ordered) = ordered.as_mut() {
            if !ordered.contains(&id) {
                ordered.push(id)
            }
        }
        visited[id.0] = false;
        Ok(())
    }

    pub fn ordered_datatype_idents(&self) -> Result<Vec<Ident>, ()> {
        let mut visited = Vec::new();
        visited.resize(self.names.len(), false);
        let mut ordered = Vec::with_capacity(visited.capacity());
        for id in self.data_types.keys() {
            let _ = self.dfs_walk(*id, &mut visited, &mut Some(&mut ordered));
        }
        Ok(ordered)
    }

    fn dfs_find_cycle(&self, id: Ident) -> Result<(), ()> {
        let mut visited = Vec::new();
        visited.resize(self.names.len(), false);
        self.dfs_walk(id, &mut visited, &mut None)
    }

    fn ensure_finite_datatype(&self, id: Ident, decl: &SyntaxDecl) -> Result<(), ValidationError> {
        self.dfs_find_cycle(id)
            .map_err(|_| ValidationError::Infinite {
                name: decl.name().to_owned(),
                location: *decl.location(),
            })
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

        for (decl, id) in decls.iter().zip(&idents) {
            mod_.define_decl(id.clone(), decl)?
        }

        for (decl, id) in decls.iter().zip(idents) {
            if decl.is_datatype() {
                mod_.ensure_finite_datatype(id, decl)?
            }
        }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::types::AtomType;

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
            let members = match &d.data_types[&Ident(0)] {
                DataType::Struct { members, .. } => members,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(members[0].name, "a");
            assert_eq!(members[1].name, "b");
            match &members[0].type_ {
                DataTypeRef::Atom(AtomType::I32) => (),
                _ => panic!("Unexpected type"),
            };
            match &members[1].type_ {
                DataTypeRef::Atom(AtomType::F32) => (),
                _ => panic!("Unexpected type"),
            };
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
            let members = match &d.data_types[&Ident(0)] {
                DataType::Enum { members, .. } => members,
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
                        args: vec![NamedMember {
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
                    DataType::Alias {
                        to: DataTypeRef::Atom(AtomType::U8),
                        attrs: Vec::new()
                    }
                )]
                .into_iter()
                .collect::<HashMap<_, _>>(),
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
