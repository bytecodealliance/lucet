use crate::error::ValidationError;
use crate::types::{
    AliasDataType, AtomType, Attr, DataType, DataTypeRef, DataTypeVariant, EnumDataType,
    EnumMember, Ident, Location, Name, StructDataType, StructMember,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
struct DataTypeIR {
    pub variant: VariantIR,
    pub attrs: Vec<Attr>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMemberIR {
    pub type_: DataTypeRef,
    pub name: String,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructIR {
    pub members: Vec<StructMemberIR>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumIR {
    pub members: Vec<EnumMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AliasIR {
    pub to: DataTypeRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VariantIR {
    Struct(StructIR),
    Enum(EnumIR),
    Alias(AliasIR),
}

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

    pub fn define(&mut self, id: Ident, variant: VariantIR, attrs: Vec<Attr>, location: Location) {
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
        ordered: &mut Vec<Ident>,
        finalized_types: &mut HashMap<Ident, DataType>,
    ) -> Result<(), ()> {
        if visited[id.0] {
            Err(())?
        }
        visited[id.0] = true;
        let dt = self.data_types.get(&id).expect("data type IR is defined");
        match &dt.variant {
            VariantIR::Struct(ref s) => {
                // First, iterate down the member to ensure this is finite, and fill in type
                // info for leaves first
                for mem in s.members.iter() {
                    if let DataTypeRef::Defined(id) = mem.type_ {
                        self.dfs_walk(id, visited, ordered, finalized_types)?;
                    };
                }
                // If finalized type information has not yet been computed, we can now compute it:
                if !finalized_types.contains_key(&id) {
                    let mut offset = 0;
                    let mut members: Vec<StructMember> = Vec::new();
                    for mem in s.members.iter() {
                        let repr_size = datatype_repr_size(&mem.type_, finalized_types)
                            .expect("datatype is defined by prior dfs_walk");

                        if let Some(prev_elem) = members.last() {
                            let prev_elem_size = prev_elem.repr_size;
                            let padding = (prev_elem_size - 1)
                                - ((offset + (prev_elem_size - 1)) % prev_elem_size);
                            offset += padding;
                        }

                        members.push(StructMember {
                            type_: mem.type_.clone(),
                            name: mem.name.clone(),
                            attrs: mem.attrs.clone(),
                            repr_size,
                            offset,
                        });
                        offset += repr_size;
                    }

                    // Struct will be aligned to the size of the first element. Structs always have
                    // at least one element.
                    let first_elem_size = members[0].repr_size;
                    let end_padding = (first_elem_size - 1)
                        - ((offset + (first_elem_size - 1)) % first_elem_size);

                    finalized_types.insert(
                        id,
                        DataType {
                            variant: DataTypeVariant::Struct(StructDataType { members }),
                            attrs: dt.attrs.clone(),
                            repr_size: offset + end_padding,
                        },
                    );
                }
            }
            VariantIR::Alias(ref a) => {
                if let DataTypeRef::Defined(pointee_id) = a.to {
                    self.dfs_walk(pointee_id, visited, ordered, finalized_types)?;
                };
                if !finalized_types.contains_key(&id) {
                    let repr_size = datatype_repr_size(&a.to, finalized_types)
                        .expect("datatype is defined by prior dfs_walk");
                    finalized_types.insert(
                        id,
                        DataType {
                            variant: DataTypeVariant::Alias(AliasDataType { to: a.to.clone() }),
                            attrs: dt.attrs.clone(),
                            repr_size,
                        },
                    );
                }
            }
            VariantIR::Enum(ref e) => {
                // No recursion to do on the dfs.
                if !finalized_types.contains_key(&id) {
                    // x86_64 ABI says enum is 32 bits wide
                    let repr_size = AtomType::U32.repr_size();
                    finalized_types.insert(
                        id,
                        DataType {
                            variant: DataTypeVariant::Enum(EnumDataType {
                                members: e.members.clone(),
                            }),
                            attrs: dt.attrs.clone(),
                            repr_size,
                        },
                    );
                }
            }
        }
        if !ordered.contains(&id) {
            ordered.push(id)
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
                // First, make sure datatypes are finite
                let mut visited = Vec::new();
                visited.resize(names.len(), false);

                self.dfs_walk(id, &mut visited, &mut ordered, &mut finalized)
                    .map_err(|_| ValidationError::Infinite {
                        name: name.name.clone(),
                        location: decl.location.clone(),
                    })?;
            }
        }
        Ok((finalized, ordered))
    }
}

fn datatype_repr_size(
    datatype_ref: &DataTypeRef,
    finalized_types: &HashMap<Ident, DataType>,
) -> Option<usize> {
    Some(match datatype_ref {
        DataTypeRef::Atom(a) => a.repr_size(),
        DataTypeRef::Defined(ref member_ident) => finalized_types.get(member_ident)?.repr_size,
    })
}
