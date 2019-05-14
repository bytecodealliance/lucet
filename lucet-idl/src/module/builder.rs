use crate::error::ValidationError;
use crate::types::{Attr, DataType, DataTypeRef, DataTypeVariant, Ident, Location, Name};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataTypeModuleBuilder {
    data_types: HashMap<Ident, DataTypeIR>,
}

impl DataTypeModuleBuilder {
    pub fn new() -> Self {
        Self {
            data_types: HashMap::new(),
        }
    }

    pub fn define(
        &mut self,
        id: Ident,
        variant: DataTypeVariant,
        attrs: Vec<Attr>,
        location: Location,
    ) {
        if let Some(prev_def) = self.data_types.insert(
            id,
            DataTypeIR {
                variant,
                attrs: attrs.clone(),
                location: location.clone(),
            },
        ) {
            panic!("id {} already defined: {:?}", id, prev_def)
        }
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
        match self
            .data_types
            .get(&id)
            .expect("data_type is defined")
            .variant
        {
            DataTypeVariant::Struct(ref s) => {
                for mem in s.members.iter() {
                    if let DataTypeRef::Defined(id) = mem.type_ {
                        self.dfs_walk(id, visited, ordered)?
                    }
                }
            }
            DataTypeVariant::Alias(ref a) => {
                if let DataTypeRef::Defined(id) = a.to {
                    self.dfs_walk(id, visited, ordered)?
                }
            }
            DataTypeVariant::Enum(_) => {}
        }
        if let Some(ordered) = ordered.as_mut() {
            if !ordered.contains(&id) {
                ordered.push(id)
            }
        }
        visited[id.0] = false;
        Ok(())
    }

    pub fn validate_datatypes(
        &self,
        names: &[Name],
    ) -> Result<(HashMap<Ident, DataType>, Vec<Ident>), ValidationError> {
        let mut finalized = HashMap::new();
        let mut ordered = Vec::new();
        // Important to iterate in name order, so error messages are consistient.
        // HashMap iteration order is not stable.
        for (ix, name) in names.iter().enumerate() {
            let id = Ident(ix);
            if let Some(decl) = self.data_types.get(&id) {
                let mut visited = Vec::new();
                visited.resize(names.len(), false);
                self.dfs_walk(id, &mut visited, &mut Some(&mut ordered))
                    .map_err(|_| ValidationError::Infinite {
                        name: name.name.clone(),
                        location: decl.location.clone(),
                    })?;
                finalized.insert(
                    id,
                    DataType {
                        variant: decl.variant.clone(),
                        attrs: decl.attrs.clone(),
                    },
                );
            }
        }
        Ok((finalized, ordered))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct DataTypeIR {
    pub variant: DataTypeVariant,
    pub attrs: Vec<Attr>,
    pub location: Location,
}
