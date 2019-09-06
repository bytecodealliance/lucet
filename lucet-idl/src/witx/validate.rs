#![allow(unused_variables)] // WIP
#![allow(unused_imports)] // WIP
#![allow(dead_code)] // WIP

use super::ast::{
    AliasDatatype, BuiltinType, Datatype, DatatypeIdent, DatatypeVariant, Definition, Document,
    Entry, EnumDatatype, FlagsDatatype, Id, ModuleDef, StructDatatype, UnionDatatype,
};
use super::parser::{
    DatatypeIdentSyntax, DeclSyntax, EnumSyntax, FieldSyntax, FlagsSyntax, IdentSyntax,
    InterfaceFuncSyntax, ModuleImportSyntax, ModuleSyntax, StructSyntax, TypedefSyntax,
    TypenameSyntax, UnionSyntax,
};
use super::Location;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Debug, Fail)]
pub enum ValidationError {
    #[fail(display = "Unknown name `{}`", name)]
    UnknownName { name: String, location: Location },
    #[fail(display = "Redefinition of name `{}`", name)]
    NameAlreadyExists {
        name: String,
        at_location: Location,
        previous_location: Location,
    },
    #[fail(
        display = "Wrong kind of name `{}`: expected {}, got {}",
        name, expected, got
    )]
    WrongKindName {
        name: String,
        location: Location,
        expected: &'static str,
        got: &'static str,
    },
    #[fail(display = "Recursive definition of name `{}`", name)]
    Recursive { name: String, location: Location },
    #[fail(display = "Invalid representation `{:?}`", repr)]
    InvalidRepr {
        repr: BuiltinType,
        location: Location,
    },
}

pub fn validate(decls: &[DeclSyntax]) -> Result<Document, ValidationError> {
    let mut validator = DeclValidation::new();
    let mut definitions = Vec::new();
    for d in decls {
        definitions.push(validator.validate_decl(&d)?);
    }

    Ok(Document {
        entries: validator.entries,
        definitions,
    })
}

struct IdentValidation {
    names: HashMap<String, Location>,
}

impl IdentValidation {
    fn new() -> Self {
        Self {
            names: HashMap::new(),
        }
    }
    fn introduce(&mut self, syntax: &IdentSyntax) -> Result<Id, ValidationError> {
        if let Some(introduced) = self.names.get(&syntax.name) {
            Err(ValidationError::NameAlreadyExists {
                name: syntax.name.clone(),
                at_location: syntax.location.clone(),
                previous_location: introduced.clone(),
            })
        } else {
            self.names
                .insert(syntax.name.clone(), syntax.location.clone());
            Ok(Id::new(&syntax.name))
        }
    }

    fn get(&self, syntax: &IdentSyntax) -> Result<Id, ValidationError> {
        if self.names.get(&syntax.name).is_some() {
            Ok(Id::new(&syntax.name))
        } else {
            Err(ValidationError::UnknownName {
                name: syntax.name.clone(),
                location: syntax.location.clone(),
            })
        }
    }
}

struct DeclValidation {
    scope: IdentValidation,
    pub entries: HashMap<Id, Entry>,
}

impl DeclValidation {
    fn new() -> Self {
        Self {
            scope: IdentValidation::new(),
            entries: HashMap::new(),
        }
    }

    fn validate_decl(&mut self, decl: &DeclSyntax) -> Result<Definition, ValidationError> {
        match decl {
            DeclSyntax::Typename(decl) => {
                let name = self.scope.introduce(&decl.ident)?;
                let variant =
                    match &decl.def {
                        TypedefSyntax::Ident(syntax) => DatatypeVariant::Alias(AliasDatatype {
                            name: name.clone(),
                            to: self.validate_datatype_ident(&syntax)?,
                        }),
                        TypedefSyntax::Enum(syntax) => DatatypeVariant::Enum(self.validate_enum(
                            &name,
                            &syntax,
                            &decl.ident.location,
                        )?),
                        TypedefSyntax::Flags(syntax) => DatatypeVariant::Flags(
                            self.validate_flags(&name, &syntax, &decl.ident.location)?,
                        ),
                        TypedefSyntax::Struct(syntax) => DatatypeVariant::Struct(
                            self.validate_struct(&name, &syntax, &decl.ident.location)?,
                        ),
                        TypedefSyntax::Union(syntax) => DatatypeVariant::Union(
                            self.validate_union(&name, &syntax, &decl.ident.location)?,
                        ),
                    };
                let rc_datatype = Rc::new(Datatype {
                    name: name.clone(),
                    variant,
                });
                self.entries
                    .insert(name, Entry::Datatype(Rc::downgrade(&rc_datatype)));
                Ok(Definition::Datatype(rc_datatype))
            }
            DeclSyntax::Module(syntax) => {
                let name = self.scope.introduce(&syntax.name)?;
                unimplemented!()
            }
        }
    }

    fn validate_datatype_ident(
        &self,
        syntax: &DatatypeIdentSyntax,
    ) -> Result<DatatypeIdent, ValidationError> {
        match syntax {
            DatatypeIdentSyntax::Builtin(b) => Ok(DatatypeIdent::Builtin(*b)),
            DatatypeIdentSyntax::Array(a) => Ok(DatatypeIdent::Array(Box::new(
                self.validate_datatype_ident(&a)?,
            ))),
            DatatypeIdentSyntax::Ident(i) => {
                let id = self.scope.get(i)?;
                match self.entries.get(&id) {
                    Some(Entry::Datatype(weak_d)) => Ok(DatatypeIdent::Ident(
                        weak_d.upgrade().expect("weak backref to defined type"),
                    )),
                    Some(e) => Err(ValidationError::WrongKindName {
                        name: i.name.clone(),
                        location: i.location.clone(),
                        expected: "datatype",
                        got: e.kind(),
                    }),
                    None => Err(ValidationError::Recursive {
                        name: i.name.clone(),
                        location: i.location.clone(),
                    }),
                }
            }
        }
    }

    fn validate_enum(
        &self,
        name: &Id,
        syntax: &EnumSyntax,
        location: &Location,
    ) -> Result<EnumDatatype, ValidationError> {
        let mut enum_scope = IdentValidation::new();
        let repr = match syntax.repr {
            // XXX fill in valid cases, factor into reusable func
            _ => Err(ValidationError::InvalidRepr {
                repr: syntax.repr.clone(),
                location: location.clone(),
            })?,
        };
        let variants = syntax
            .members
            .iter()
            .map(|i| enum_scope.introduce(i))
            .collect::<Result<Vec<Id>, _>>()?;

        Ok(EnumDatatype {
            name: name.clone(),
            repr,
            variants,
        })
    }

    fn validate_flags(
        &self,
        name: &Id,
        syntax: &FlagsSyntax,
        location: &Location,
    ) -> Result<FlagsDatatype, ValidationError> {
        unimplemented!()
    }

    fn validate_struct(
        &self,
        name: &Id,
        syntax: &StructSyntax,
        location: &Location,
    ) -> Result<StructDatatype, ValidationError> {
        unimplemented!()
    }

    fn validate_union(
        &self,
        name: &Id,
        syntax: &UnionSyntax,
        location: &Location,
    ) -> Result<UnionDatatype, ValidationError> {
        unimplemented!()
    }
}
