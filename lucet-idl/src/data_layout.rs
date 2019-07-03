use crate::error::ValidationError;
use crate::types::{
    AliasDataType, AtomType, DataType, DataTypeRef, DataTypeVariant, EnumDataType, EnumMember,
    Ident, Location, Name, StructDataType, StructMember,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
struct DataTypeIR {
    pub variant: VariantIR,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMemberIR {
    pub type_: DataTypeRef,
    pub name: String,
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

    pub fn define(&mut self, id: Ident, variant: VariantIR, location: Location) {
        if let Some(prev_def) = self.data_types.insert(
            id,
            DataTypeIR {
                variant,
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
                    let mut struct_align = 1;
                    let mut members: Vec<StructMember> = Vec::new();
                    for mem in s.members.iter() {
                        let (repr_size, align) =
                            datatype_repr_size_align(&mem.type_, finalized_types)
                                .expect("datatype is defined by prior dfs_walk");

                        offset = align_to(offset, align);
                        struct_align = ::std::cmp::max(struct_align, align);

                        members.push(StructMember {
                            type_: mem.type_.clone(),
                            name: mem.name.clone(),
                            offset,
                        });
                        offset += repr_size;
                    }

                    let repr_size = align_to(offset, struct_align);

                    finalized_types.insert(
                        id,
                        DataType {
                            variant: DataTypeVariant::Struct(StructDataType { members }),
                            repr_size,
                            align: struct_align,
                        },
                    );
                }
            }
            VariantIR::Alias(ref a) => {
                if let DataTypeRef::Defined(pointee_id) = a.to {
                    self.dfs_walk(pointee_id, visited, ordered, finalized_types)?;
                };
                if !finalized_types.contains_key(&id) {
                    let (repr_size, align) = datatype_repr_size_align(&a.to, finalized_types)
                        .expect("datatype is defined by prior dfs_walk");
                    finalized_types.insert(
                        id,
                        DataType {
                            variant: DataTypeVariant::Alias(AliasDataType { to: a.to.clone() }),
                            repr_size,
                            align,
                        },
                    );
                }
            }
            VariantIR::Enum(ref e) => {
                // No recursion to do on the dfs.
                if !finalized_types.contains_key(&id) {
                    // x86_64 ABI says enum is 32 bits wide
                    let repr_size = AtomType::U32.repr_size();
                    let align = repr_size;
                    finalized_types.insert(
                        id,
                        DataType {
                            variant: DataTypeVariant::Enum(EnumDataType {
                                members: e.members.clone(),
                            }),
                            repr_size,
                            align,
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

fn datatype_repr_size_align(
    datatype_ref: &DataTypeRef,
    finalized_types: &HashMap<Ident, DataType>,
) -> Option<(usize, usize)> {
    let (size, align) = match datatype_ref {
        DataTypeRef::Atom(a) => {
            let s = a.repr_size();
            (s, s)
        }
        DataTypeRef::Defined(ref member_ident) => {
            let t = finalized_types.get(member_ident)?;
            (t.repr_size, t.align)
        }
    };
    assert!(size > 0);
    assert!(align > 0);
    Some((size, align))
}
fn align_to(offs: usize, alignment: usize) -> usize {
    offs + alignment - 1 - ((offs + alignment - 1) % alignment)
}

#[cfg(test)]
mod align_test {
    use super::align_to;
    #[test]
    fn align_test() {
        assert_eq!(0, align_to(0, 1));
        assert_eq!(0, align_to(0, 2));
        assert_eq!(0, align_to(0, 4));
        assert_eq!(0, align_to(0, 8));

        assert_eq!(1, align_to(1, 1));
        assert_eq!(2, align_to(1, 2));
        assert_eq!(4, align_to(1, 4));
        assert_eq!(8, align_to(1, 8));

        assert_eq!(2, align_to(2, 1));
        assert_eq!(2, align_to(2, 2));
        assert_eq!(4, align_to(2, 4));
        assert_eq!(8, align_to(2, 8));

        assert_eq!(5, align_to(5, 1));
        assert_eq!(6, align_to(5, 2));
        assert_eq!(8, align_to(5, 4));
        assert_eq!(8, align_to(5, 8));
    }
}
