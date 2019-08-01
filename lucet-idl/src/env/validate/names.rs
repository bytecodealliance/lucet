use crate::env::repr::{DatatypeIdent, DatatypeIx, FuncIx, ModuleIx};
use crate::error::ValidationError;
use crate::parser::SyntaxTypeRef;
use crate::types::Location;
use cranelift_entity::PrimaryMap;
use std::collections::HashMap;

pub struct ModNamesBuilder {
    pub module: ModuleIx,
    pub funcs: PrimaryMap<FuncIx, String>,
    pub types: PrimaryMap<DatatypeIx, String>,
    names: HashMap<String, (ModContentIx, Location)>,
}

impl ModNamesBuilder {
    pub fn new(module: ModuleIx) -> Self {
        Self {
            module,
            names: HashMap::new(),
            funcs: PrimaryMap::new(),
            types: PrimaryMap::new(),
        }
    }

    pub fn introduce_datatype(
        &mut self,
        name: &str,
        location: &Location,
    ) -> Result<(), ValidationError> {
        if let Some((_, prev_loc)) = self.names.get(name) {
            Err(ValidationError::NameAlreadyExists {
                name: name.to_owned(),
                at_location: *location,
                previous_location: *prev_loc,
            })?;
        }
        let ix = self.types.push(name.to_owned());
        self.names
            .insert(name.to_owned(), (ModContentIx::Datatype(ix), *location));
        Ok(())
    }

    pub fn introduce_function(
        &mut self,
        name: &str,
        location: &Location,
    ) -> Result<(), ValidationError> {
        if let Some((_, prev_loc)) = self.names.get(name) {
            Err(ValidationError::NameAlreadyExists {
                name: name.to_owned(),
                at_location: *location,
                previous_location: *prev_loc,
            })?;
        }
        let ix = self.funcs.push(name.to_owned());
        self.names
            .insert(name.to_owned(), (ModContentIx::Func(ix), *location));
        Ok(())
    }

    pub fn datatype_id_from_syntax(
        &self,
        syntax: &SyntaxTypeRef,
    ) -> Result<DatatypeIdent, ValidationError> {
        match syntax {
            SyntaxTypeRef::Atom { atom, .. } => Ok(atom.datatype_id()),
            SyntaxTypeRef::Name { name, location } => match self.names.get(name) {
                Some((ModContentIx::Datatype(ix), _loc)) => {
                    Ok(DatatypeIdent::new(self.module, *ix))
                }
                Some((_, bound_loc)) => Err(ValidationError::NameSortError {
                    name: name.to_owned(),
                    use_location: *location,
                    bound_location: *bound_loc,
                }),
                None => Err(ValidationError::NameNotFound {
                    name: name.to_owned(),
                    use_location: *location,
                }),
            },
        }
    }

    pub fn datatype_from_name(&self, name: &str) -> Option<DatatypeIx> {
        self.names.get(name).and_then(|(ix, _)| ix.datatype())
    }

    pub fn func_from_name(&self, name: &str) -> Option<FuncIx> {
        self.names.get(name).and_then(|(ix, _)| ix.func())
    }
}

enum ModContentIx {
    Datatype(DatatypeIx),
    Func(FuncIx),
}

impl ModContentIx {
    fn datatype(&self) -> Option<DatatypeIx> {
        match self {
            ModContentIx::Datatype(d) => Some(*d),
            _ => None,
        }
    }
    fn func(&self) -> Option<FuncIx> {
        match self {
            ModContentIx::Func(f) => Some(*f),
            _ => None,
        }
    }
}
