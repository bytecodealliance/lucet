use super::parser::{SyntaxDecl, SyntaxRef};
use super::types::{AtomType, Attr, Location};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct DatatypeId(pub usize);

impl fmt::Display for DatatypeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DatatypeRef {
    Defined(DatatypeId),
    Atom(AtomType),
    Ptr(Box<DatatypeRef>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NamedMember<R> {
    pub type_: R,
    pub name: String,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Datatype {
    Struct {
        members: Vec<NamedMember<DatatypeRef>>,
        attrs: Vec<Attr>,
    },
    TaggedUnion {
        members: Vec<NamedMember<Option<DatatypeRef>>>,
        attrs: Vec<Attr>,
    },
    Enum {
        members: Vec<NamedMember<()>>,
        attrs: Vec<Attr>,
    },
    Alias {
        to: DatatypeRef,
        attrs: Vec<Attr>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Name {
    pub name: String,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataDescription {
    pub names: Vec<Name>,
    pub datatypes: HashMap<usize, Datatype>,
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl Error for ValidationError {
    fn description(&self) -> &str {
        "Validation error"
    }
}

impl DataDescription {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            datatypes: HashMap::new(),
        }
    }

    fn introduce_name(
        &mut self,
        name: &str,
        location: &Location,
    ) -> Result<DatatypeId, ValidationError> {
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
            Ok(DatatypeId(id))
        }
    }

    fn id_for_name(&self, name: &str) -> Option<DatatypeId> {
        for (id, n) in self.names.iter().enumerate() {
            if n.name == name {
                return Some(DatatypeId(id));
            }
        }
        None
    }

    fn get_ref(&self, syntax_ref: &SyntaxRef) -> Result<DatatypeRef, ValidationError> {
        match syntax_ref {
            SyntaxRef::Atom { atom, .. } => Ok(DatatypeRef::Atom(*atom)),
            SyntaxRef::Ptr { to, .. } => Ok(DatatypeRef::Ptr(Box::new(self.get_ref(&to)?))),
            SyntaxRef::Name { name, location } => match self.id_for_name(name) {
                Some(id) => Ok(DatatypeRef::Defined(id)),
                None => Err(ValidationError::NameNotFound {
                    name: name.clone(),
                    use_location: *location,
                }),
            },
        }
    }

    fn define_datatype(&mut self, id: DatatypeId, dt: Datatype) {
        if let Some(prev_def) = self.datatypes.insert(id.0, dt) {
            panic!("id {} already defined: {:?}", id, prev_def)
        }
    }

    fn define_decl(&mut self, id: DatatypeId, decl: &SyntaxDecl) -> Result<(), ValidationError> {
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
                    // Get the DatatypeRef for the member, which ensures that it refers only to
                    // defined types:
                    let type_ = self.get_ref(&mem.type_)?;
                    // build the struct with this as the member:
                    dtype_members.push(NamedMember {
                        type_,
                        name: mem.name.clone(),
                        attrs: mem.attrs.clone(),
                    })
                }
                self.define_datatype(
                    id,
                    Datatype::Struct {
                        members: dtype_members,
                        attrs: attrs.clone(),
                    },
                )
            }
            SyntaxDecl::TaggedUnion {
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
                    // Get the DatatypeRef for the member, which ensures that it refers only to
                    // defined types:
                    let type_ = if let Some(ref t) = var.type_ {
                        Some(self.get_ref(t)?)
                    } else {
                        None
                    };
                    // build the struct with this as the member:
                    dtype_members.push(NamedMember {
                        type_,
                        name: var.name.clone(),
                        attrs: var.attrs.clone(),
                    })
                }
                self.define_datatype(
                    id,
                    Datatype::TaggedUnion {
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
                self.define_datatype(
                    id,
                    Datatype::Enum {
                        members: dtype_members,
                        attrs: attrs.clone(),
                    },
                )
            }
            SyntaxDecl::Alias { what, attrs, .. } => {
                let to = self.get_ref(what)?;
                self.define_datatype(
                    id,
                    Datatype::Alias {
                        to,
                        attrs: attrs.clone(),
                    },
                );
            }
        }
        Ok(())
    }

    fn dfs_walk(
        &self,
        id: DatatypeId,
        visited: &mut [bool],
        ordered: &mut Option<&mut Vec<DatatypeId>>,
    ) -> Result<(), ()> {
        if visited[id.0] {
            Err(())?
        }
        visited[id.0] = true;
        match self.datatypes.get(&id.0).expect("datatype is defined") {
            Datatype::Struct { members, .. } => {
                for mem in members {
                    if let DatatypeRef::Defined(id) = mem.type_ {
                        self.dfs_walk(id, visited, ordered)?
                    }
                }
            }
            Datatype::TaggedUnion { members, .. } => {
                for mem in members {
                    if let Some(DatatypeRef::Defined(id)) = mem.type_ {
                        self.dfs_walk(id, visited, ordered)?
                    }
                }
            }
            Datatype::Alias { to, .. } => {
                if let DatatypeRef::Defined(id) = to {
                    self.dfs_walk(*id, visited, ordered)?
                }
            }
            Datatype::Enum { .. } => {}
        }
        if let Some(ordered) = ordered.as_mut() {
            if !ordered.contains(&id) {
                ordered.push(id)
            }
        }
        visited[id.0] = false;
        Ok(())
    }

    pub fn ordered_dependencies(&self) -> Result<Vec<DatatypeId>, ()> {
        let mut visited = Vec::new();
        visited.resize(self.names.len(), false);
        let mut ordered = Vec::with_capacity(visited.capacity());
        for id in self.datatypes.keys() {
            let _ = self.dfs_walk(DatatypeId(*id), &mut visited, &mut Some(&mut ordered));
        }
        Ok(ordered)
    }

    fn dfs_find_cycle(&self, id: DatatypeId) -> Result<(), ()> {
        let mut visited = Vec::new();
        visited.resize(self.names.len(), false);
        self.dfs_walk(id, &mut visited, &mut None)
    }

    fn ensure_finite(&self, id: DatatypeId, decl: &SyntaxDecl) -> Result<(), ValidationError> {
        self.dfs_find_cycle(id)
            .map_err(|_| ValidationError::Infinite {
                name: decl.name().to_owned(),
                location: *decl.location(),
            })
    }

    pub fn validate(decls: &[SyntaxDecl]) -> Result<DataDescription, ValidationError> {
        let mut desc = Self::new();
        let mut idents: Vec<DatatypeId> = Vec::new();
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
}

#[cfg(test)]
mod tests {
    use super::super::parser::Parser;
    use super::*;

    fn data_description(syntax: &str) -> Result<DataDescription, ValidationError> {
        let mut parser = Parser::new(syntax);
        let decls = parser.match_decls().expect("parses");
        DataDescription::validate(&decls)
    }

    #[test]
    fn structs() {
        assert!(data_description("struct foo { a: i32}").is_ok());
        assert!(data_description("struct foo { a: i32, b: f32 }").is_ok());

        {
            let d = data_description("struct foo { a: i32, b: f32 }").unwrap();
            let members = match &d.datatypes[&0] {
                Datatype::Struct { members, .. } => members,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(members[0].name, "a");
            assert_eq!(members[1].name, "b");
            match &members[0].type_ {
                DatatypeRef::Atom(AtomType::I32) => (),
                _ => panic!("Unexpected type"),
            };
            match &members[1].type_ {
                DatatypeRef::Atom(AtomType::F32) => (),
                _ => panic!("Unexpected type"),
            };
        }

        // Refer to a struct defined previously:
        assert!(data_description("struct foo { a: i32, b: f64 } struct bar { a: foo }").is_ok());
        // Refer to a struct defined afterwards:
        assert!(data_description("struct foo { a: i32, b: bar} struct bar { a: i32 }").is_ok());

        // Refer to itself
        assert!(data_description("struct list { next: *list, thing: i32 }").is_ok());

        // No members
        assert_eq!(
            data_description("struct foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        // Duplicate member in struct
        assert_eq!(
            data_description("struct foo { \na: i32, \na: f64}")
                .err()
                .unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );

        // Duplicate definition of struct
        assert_eq!(
            data_description("struct foo { a: i32 }\nstruct foo { a: i32 } ")
                .err()
                .unwrap(),
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );

        // Refer to type that is not declared
        assert_eq!(
            data_description("struct foo { \nb: bar }").err().unwrap(),
            ValidationError::NameNotFound {
                name: "bar".to_owned(),
                use_location: Location { line: 2, column: 3 },
            }
        );
    }

    #[test]
    fn tagged_unions() {
        assert!(data_description("taggedunion foo { a: () }").is_ok());
        assert!(data_description("taggedunion foo { a: i32 }").is_ok());
        assert!(data_description("taggedunion foo { a: i32, b: f32 }").is_ok());
        assert!(data_description("taggedunion foo { a: i32, b: () }").is_ok());

        {
            let d = data_description("taggedunion foo { a: i32, b: () }").unwrap();
            let members = match &d.datatypes[&0] {
                Datatype::TaggedUnion { members, .. } => members,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(members[0].name, "a");
            assert_eq!(members[1].name, "b");
            match &members[0].type_ {
                Some(DatatypeRef::Atom(AtomType::I32)) => (),
                _ => panic!("Unexpected type"),
            };
            match &members[1].type_ {
                None => (),
                _ => panic!("Unexpected type"),
            };
        }

        // Recursive
        assert!(data_description("taggedunion cons { succ: *cons, nil: () }").is_ok());

        // Refer to a taggedunion defined previously:
        assert!(
            data_description("taggedunion foo { a: i32, b: f64 } taggedunion bar { a: foo }")
                .is_ok()
        );
        // Refer to a taggedunion defined afterwards:
        assert!(
            data_description("taggedunion foo { a: i32, b: bar} taggedunion bar { a: i32 }")
                .is_ok()
        );

        // No members
        assert_eq!(
            data_description("taggedunion foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        // Duplicate member in taggedunion
        assert_eq!(
            data_description("taggedunion foo { \na: i32, \na: f64}")
                .err()
                .unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );

        // Duplicate definition of name "foo"
        assert_eq!(
            data_description("taggedunion foo { a: i32 }\nstruct foo { a: i32 } ")
                .err()
                .unwrap(),
            ValidationError::NameAlreadyExists {
                name: "foo".to_owned(),
                at_location: Location { line: 2, column: 0 },
                previous_location: Location { line: 1, column: 0 },
            }
        );

        // Refer to type that is not declared
        assert_eq!(
            data_description("taggedunion foo { \nb: bar }")
                .err()
                .unwrap(),
            ValidationError::NameNotFound {
                name: "bar".to_owned(),
                use_location: Location { line: 2, column: 3 },
            }
        );
    }

    #[test]
    fn enums() {
        assert!(data_description("enum foo { a }").is_ok());
        assert!(data_description("enum foo { a, b }").is_ok());

        {
            let d = data_description("enum foo { a, b }").unwrap();
            let members = match &d.datatypes[&0] {
                Datatype::Enum { members, .. } => members,
                _ => panic!("Unexpected type"),
            };
            assert_eq!(members[0].name, "a");
            assert_eq!(members[1].name, "b");
        }

        // No members
        assert_eq!(
            data_description("enum foo {}").err().unwrap(),
            ValidationError::Empty {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        // Duplicate member in enum
        assert_eq!(
            data_description("enum foo { \na,\na }").err().unwrap(),
            ValidationError::NameAlreadyExists {
                name: "a".to_owned(),
                at_location: Location { line: 3, column: 0 },
                previous_location: Location { line: 2, column: 0 },
            }
        );

        // Duplicate definition of enum
        assert_eq!(
            data_description("enum foo { a }\nenum foo { a } ")
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
    fn aliases() {
        assert!(data_description("type foo = i32").is_ok());
        assert!(data_description("type foo = *f64").is_ok());
        assert!(data_description("type foo = ************f64").is_ok());

        assert!(data_description("type foo = *bar\nenum bar { a }").is_ok());

        assert!(
            data_description("type link = *list\nstruct list { next: link, thing: i32 }").is_ok()
        );
    }

    #[test]
    fn infinite() {
        assert_eq!(
            data_description("type foo = bar\ntype bar = foo")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            data_description("type foo = bar\nstruct bar { a: foo }")
                .err()
                .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );

        assert_eq!(
            data_description(
                "type foo = bar\nstruct bar { a: baz }\ntaggedunion baz { c: i32, e: foo }"
            )
            .err()
            .unwrap(),
            ValidationError::Infinite {
                name: "foo".to_owned(),
                location: Location { line: 1, column: 0 },
            }
        );
    }
}
