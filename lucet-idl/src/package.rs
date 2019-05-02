use crate::parser::{SyntaxDecl, SyntaxRef};
use crate::types::{AtomType, Attr, Location};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct DataTypeId(pub usize);

impl fmt::Display for DataTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DataTypeRef {
    Defined(DataTypeId),
    Atom(AtomType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NamedMember<R> {
    pub type_: R,
    pub name: String,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DataType {
    Struct {
        members: Vec<NamedMember<DataTypeRef>>,
        attrs: Vec<Attr>,
    },
    Enum {
        members: Vec<NamedMember<()>>,
        attrs: Vec<Attr>,
    },
    Alias {
        to: DataTypeRef,
        attrs: Vec<Attr>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Name {
    pub name: String,
    pub location: Location,
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Package {
    pub names: Vec<Name>,
    pub data_types: HashMap<usize, DataType>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValidationError {
    NameAlreadyExists {
        name: String,
        at_location: Location,
        previous_location: Location,
    },
    NameNotFound {
        name: String,
        use_location: Location,
    },
    Empty {
        name: String,
        location: Location,
    },
    Infinite {
        name: String,
        location: Location,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::NameAlreadyExists {
                name,
                at_location,
                previous_location,
            } => write!(
                f,
                "Redefinition of name {} at line {} - previous definition was at line {}",
                name, at_location.line, previous_location.line
            ),
            ValidationError::NameNotFound { name, use_location } => {
                write!(f, "Name {} not found at line {}", name, use_location.line)
            }
            ValidationError::Empty { name, location } => {
                write!(f, "Empty definition for {} at line {}", name, location.line)
            }
            ValidationError::Infinite { name, location } => write!(
                f,
                "Circular reference for {} at line {}",
                name, location.line
            ),
        }
    }
}

/// A convenient structure holding a data type, its name and
/// its internal IDL representation
#[derive(Debug, Clone)]
pub struct DataTypeEntry<'t> {
    pub id: DataTypeId,
    pub name: &'t Name,
    pub data_type: &'t DataType,
}

impl Error for ValidationError {
    fn description(&self) -> &str {
        "Validation error"
    }
}

impl Package {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            data_types: HashMap::new(),
        }
    }

    fn introduce_name(
        &mut self,
        name: &str,
        location: &Location,
    ) -> Result<DataTypeId, ValidationError> {
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
            Ok(DataTypeId(id))
        }
    }

    fn id_for_name(&self, name: &str) -> Option<DataTypeId> {
        for (id, n) in self.names.iter().enumerate() {
            if n.name == name {
                return Some(DataTypeId(id));
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

    fn define_data_type(&mut self, id: DataTypeId, dt: DataType) {
        if let Some(prev_def) = self.data_types.insert(id.0, dt) {
            panic!("id {} already defined: {:?}", id, prev_def)
        }
    }

    fn define_decl(&mut self, id: DataTypeId, decl: &SyntaxDecl) -> Result<(), ValidationError> {
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
            SyntaxDecl::Module{ .. } => {
                unimplemented!()
            }
        }
        Ok(())
    }

    fn dfs_walk(
        &self,
        id: DataTypeId,
        visited: &mut [bool],
        ordered: &mut Option<&mut Vec<DataTypeId>>,
    ) -> Result<(), ()> {
        if visited[id.0] {
            Err(())?
        }
        visited[id.0] = true;
        match self.data_types.get(&id.0).expect("data_type is defined") {
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

    pub fn ordered_dependencies(&self) -> Result<Vec<DataTypeId>, ()> {
        let mut visited = Vec::new();
        visited.resize(self.names.len(), false);
        let mut ordered = Vec::with_capacity(visited.capacity());
        for id in self.data_types.keys() {
            let _ = self.dfs_walk(DataTypeId(*id), &mut visited, &mut Some(&mut ordered));
        }
        Ok(ordered)
    }

    fn dfs_find_cycle(&self, id: DataTypeId) -> Result<(), ()> {
        let mut visited = Vec::new();
        visited.resize(self.names.len(), false);
        self.dfs_walk(id, &mut visited, &mut None)
    }

    fn ensure_finite(&self, id: DataTypeId, decl: &SyntaxDecl) -> Result<(), ValidationError> {
        self.dfs_find_cycle(id)
            .map_err(|_| ValidationError::Infinite {
                name: decl.name().to_owned(),
                location: *decl.location(),
            })
    }

    pub fn from_declarations(decls: &[SyntaxDecl]) -> Result<Package, ValidationError> {
        let mut desc = Self::new();
        let mut idents: Vec<DataTypeId> = Vec::new();
        for decl in decls {
            idents.push(desc.introduce_name(decl.name(), decl.location())?)
        }

        for (decl, id) in decls.iter().zip(&idents) {
            desc.define_decl(id.clone(), decl)?
        }

        for (decl, id) in decls.iter().zip(idents) {
            desc.ensure_finite(id, decl)?
        }

        Ok(desc)
    }

    /// Retrieve information about a data type given its identifier
    pub fn get_datatype(&self, id: DataTypeId) -> DataTypeEntry<'_> {
        let name = &self.names[id.0];
        let data_type = &self.data_types[&id.0];
        DataTypeEntry {
            id,
            name,
            data_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::Parser;
    use super::*;

    fn pkg(syntax: &str) -> Result<Package, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_decls().expect("parses");
        Package::from_declarations(&decls)
    }

    #[test]
    fn structs_basic() {
        assert!(pkg("struct foo { a: i32}").is_ok());
        assert!(pkg("struct foo { a: i32, b: f32 }").is_ok());

    }

    #[test]
    fn struct_two_atoms() {
        {
            let d = pkg("struct foo { a: i32, b: f32 }").unwrap();
            let members = match &d.data_types[&0] {
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
        assert!(pkg("struct foo { a: i32, b: f64 } struct bar { a: foo }").is_ok());
    }

    #[test]
    fn struct_next_definition() {
        // Refer to a struct defined afterwards:
        assert!(pkg("struct foo { a: i32, b: bar} struct bar { a: i32 }").is_ok());

    }

    #[test]
    fn struct_self_referential() {
        // Refer to itself
        assert!(pkg("struct list { next: list, thing: i32 }").is_err());

    }

    #[test]
    fn struct_empty() {
        // No members
        assert_eq!(
            pkg("struct foo {}").err().unwrap(),
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
            pkg("struct foo { \na: i32, \na: f64}").err().unwrap(),
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
            pkg("struct foo { a: i32 }\nstruct foo { a: i32 } ")
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
            pkg("struct foo { \nb: bar }").err().unwrap(),
            ValidationError::NameNotFound {
                name: "bar".to_owned(),
                use_location: Location { line: 2, column: 3 },
            }
        );
    }

    #[test]
    fn enums() {
        assert!(pkg("enum foo { a }").is_ok());
        assert!(pkg("enum foo { a, b }").is_ok());

        {
            let d = pkg("enum foo { a, b }").unwrap();
            let members = match &d.data_types[&0] {
                DataType::Enum { members, .. } => members,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(members[0].name, "a");
            assert_eq!(members[1].name, "b");
        }

        // No members
        assert_eq!(
            pkg("enum foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        // Duplicate member in enum
        assert_eq!(
            pkg("enum foo { \na,\na }").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );

        // Duplicate definition of enum
        assert_eq!(
            pkg("enum foo { a }\nenum foo { a } ").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn aliases() {
        assert!(pkg("type foo = i32").is_ok());
        assert!(pkg("type foo = f64").is_ok());
        assert!(pkg("type foo = u8").is_ok());

        assert!(pkg("type foo = bar\nenum bar { a }").is_ok());

        assert!(pkg("type link = u32\nstruct list { next: link, thing: i32 }").is_ok());
    }

    #[test]
    fn infinite() {
        assert_eq!(
            pkg("type foo = bar\ntype bar = foo").err().unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            pkg("type foo = bar\nstruct bar { a: foo }")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            pkg("type foo = bar\nstruct bar { a: baz }\nstruct baz { c: i32, e: foo }")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );
    }
}
