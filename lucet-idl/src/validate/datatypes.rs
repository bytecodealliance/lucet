use super::names::ModNamesBuilder;
use crate::parser::{
    EnumVariant as EnumVariantSyntax, StructMember as StructMemberSyntax, SyntaxIdent,
};
use crate::repr::{
    AliasDatatypeRepr, DatatypeIdent, DatatypeIx, DatatypeRepr, DatatypeVariantRepr,
    EnumDatatypeRepr, EnumMemberRepr, ModuleDatatypesRepr, Package, StructDatatypeRepr,
    StructMemberRepr,
};
use crate::{AtomType, Location, MemArea, ValidationError};
use cranelift_entity::{PrimaryMap, SecondaryMap};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
struct DatatypeIR {
    variant: VariantIR,
    location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct StructMemberIR {
    type_: DatatypeIdent,
    name: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct StructIR {
    members: Vec<StructMemberIR>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct EnumIR {
    members: Vec<EnumMemberRepr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct AliasIR {
    to: DatatypeIdent,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum VariantIR {
    Struct(StructIR),
    Enum(EnumIR),
    Alias(AliasIR),
}

#[derive(Clone)]
pub struct DatatypeModuleBuilder<'a> {
    env: &'a Package,
    names: &'a ModNamesBuilder,
    types: PrimaryMap<DatatypeIx, DatatypeIR>,
}

impl<'a> DatatypeModuleBuilder<'a> {
    pub fn new(env: &'a Package, names: &'a ModNamesBuilder) -> Self {
        Self {
            env,
            names,
            types: PrimaryMap::new(),
        }
    }

    pub fn introduce_struct(
        &mut self,
        name: &str,
        members_syntax: &[StructMemberSyntax],
        location: Location,
    ) -> Result<(), ValidationError> {
        let ix = self
            .names
            .datatype_from_name(name)
            .expect("name is introduced");
        if members_syntax.is_empty() {
            Err(ValidationError::Empty {
                name: name.to_owned(),
                location,
            })?
        }

        let mut uniq_membs = HashMap::new();
        let mut members = Vec::new();
        for mem in members_syntax {
            // Ensure that each member name is unique:
            if let Some(existing) = uniq_membs.insert(mem.name.to_owned(), mem) {
                Err(ValidationError::NameAlreadyExists {
                    name: mem.name.to_owned(),
                    at_location: mem.location,
                    previous_location: existing.location,
                })?
            }
            // Get the DatatypeIdent for the member, which ensures that it refers only to
            // defined types:
            let type_ = self.names.datatype_id_from_syntax(&mem.type_)?;
            // build the struct with this as the member:
            members.push(StructMemberIR {
                type_,
                name: mem.name.to_owned(),
            });
        }
        self.define_datatype(
            ix,
            DatatypeIR {
                variant: VariantIR::Struct(StructIR { members }),
                location,
            },
        );
        Ok(())
    }

    pub fn introduce_enum(
        &mut self,
        name: &str,
        variants: &[EnumVariantSyntax],
        location: Location,
    ) -> Result<(), ValidationError> {
        let ix = self
            .names
            .datatype_from_name(name)
            .expect("name is introduced");
        if variants.is_empty() {
            Err(ValidationError::Empty {
                name: name.to_owned(),
                location,
            })?
        }

        let mut uniq_vars = HashMap::new();
        let mut members = Vec::new();
        for var in variants {
            // Ensure that each member name is unique:
            if let Some(existing) = uniq_vars.insert(var.name.clone(), var) {
                Err(ValidationError::NameAlreadyExists {
                    name: var.name.to_owned(),
                    at_location: var.location,
                    previous_location: existing.location,
                })?
            }
            // build the enum with this as the member:
            members.push(EnumMemberRepr {
                name: var.name.to_owned(),
            })
        }
        self.define_datatype(
            ix,
            DatatypeIR {
                variant: VariantIR::Enum(EnumIR { members }),
                location,
            },
        );
        Ok(())
    }

    pub fn introduce_alias(
        &mut self,
        name: &str,
        dest: &SyntaxIdent,
        location: Location,
    ) -> Result<(), ValidationError> {
        let ix = self
            .names
            .datatype_from_name(name)
            .expect("name is introduced");
        let to = self.names.datatype_id_from_syntax(dest)?;
        self.define_datatype(
            ix,
            DatatypeIR {
                variant: VariantIR::Alias(AliasIR { to }),
                location,
            },
        );
        Ok(())
    }

    fn define_datatype(&mut self, ix: DatatypeIx, ir: DatatypeIR) {
        let type_ix = self.types.push(ir);
        assert_eq!(
            ix, type_ix,
            "datatypes must be introduced in the same order as their names"
        );
    }

    pub fn build(self) -> Result<ModuleDatatypesRepr, ValidationError> {
        let mut finalized = FinalizedTypes::new(self.names.types.len());

        let mut ordered = Vec::new();
        for (ix, name) in self.names.types.iter() {
            let decl = self
                .types
                .get(ix)
                .expect("all datatypes declared were defined");

            // Depth first search through datatypes will return an error if they
            // are infinite, by marking all visited datatypes in this map:
            let mut visited = SecondaryMap::new();
            visited.resize(self.names.types.len());

            self.dfs_walk(ix, &mut visited, &mut ordered, &mut finalized)
                .map_err(|_| ValidationError::Infinite {
                    name: name.clone(),
                    location: decl.location,
                })?;
        }

        let datatypes = finalized.build();

        assert_eq!(
            self.names.types.len(),
            datatypes.len(),
            "each datatype defined"
        );
        assert_eq!(
            datatypes.len(),
            ordered.len(),
            "is each datatype present in topological sort? lengths dont match"
        );

        Ok(ModuleDatatypesRepr {
            names: self.names.types.clone(),
            datatypes,
            topological_order: ordered,
        })
    }

    fn dfs_walk(
        &self,
        ix: DatatypeIx,
        visited: &mut SecondaryMap<DatatypeIx, bool>,
        ordered: &mut Vec<DatatypeIx>,
        finalized_types: &mut FinalizedTypes,
    ) -> Result<(), ()> {
        // Ensure that dfs terminates:
        if visited[ix] {
            Err(())?
        }
        visited[ix] = true;

        let dt = self.types.get(ix).expect("data type IR is defined");

        match &dt.variant {
            VariantIR::Struct(ref s) => {
                // First, iterate down the member to ensure this is finite, and fill in type
                // info for leaves first.
                // IMPORTANT: assumes any type defined outside this module is an atom!
                for mem in s.members.iter() {
                    if mem.type_.module == self.names.module {
                        self.dfs_walk(mem.type_.datatype, visited, ordered, finalized_types)?;
                    }
                }
                // If finalized type information has not yet been computed, we can now compute it:
                if !finalized_types.is_defined(ix) {
                    let mut offset = 0;
                    let mut struct_align = 1;
                    let mut members: Vec<StructMemberRepr> = Vec::new();
                    for mem in s.members.iter() {
                        let (mem_size, align) =
                            self.datatype_size_align(mem.type_, finalized_types);

                        offset = align_to(offset, align);
                        struct_align = ::std::cmp::max(struct_align, align);

                        members.push(StructMemberRepr {
                            type_: mem.type_.clone(),
                            name: mem.name.clone(),
                            offset,
                        });
                        offset += mem_size;
                    }

                    let mem_size = align_to(offset, struct_align);

                    finalized_types.define(
                        ix,
                        DatatypeRepr {
                            variant: DatatypeVariantRepr::Struct(StructDatatypeRepr { members }),
                            mem_size,
                            mem_align: struct_align,
                        },
                    );
                }
            }
            VariantIR::Alias(ref a) => {
                // Iterate down the pointer to ensure this is finite, and fill in type
                // info for pointee first.
                // IMPORTANT: assumes any type defined outside this module is an atom!
                if a.to.module == self.names.module {
                    self.dfs_walk(a.to.datatype, visited, ordered, finalized_types)?;
                }

                // If finalized type information has not yet been computed, we can now compute it:
                if !finalized_types.is_defined(ix) {
                    let (mem_size, mem_align) = self.datatype_size_align(a.to, finalized_types);
                    finalized_types.define(
                        ix,
                        DatatypeRepr {
                            variant: DatatypeVariantRepr::Alias(AliasDatatypeRepr {
                                to: a.to.clone(),
                            }),
                            mem_size,
                            mem_align,
                        },
                    );
                }
            }
            VariantIR::Enum(ref e) => {
                // No recursion to do on the dfs.
                if !finalized_types.is_defined(ix) {
                    // x86_64 ABI says enum is 32 bits wide
                    let mem_size = AtomType::U32.mem_size();
                    let mem_align = mem_size;
                    finalized_types.define(
                        ix,
                        DatatypeRepr {
                            variant: DatatypeVariantRepr::Enum(EnumDatatypeRepr {
                                members: e.members.clone(),
                            }),
                            mem_size,
                            mem_align,
                        },
                    );
                }
            }
        }
        if !ordered.contains(&ix) {
            ordered.push(ix)
        }

        // dfs: allowed to visit here again
        visited[ix] = false;
        Ok(())
    }

    fn datatype_size_align(
        &self,
        id: DatatypeIdent,
        finalized_types: &FinalizedTypes,
    ) -> (usize, usize) {
        let (size, align) = if id.module == self.names.module {
            finalized_types
                .size_align(id.datatype)
                .expect("looking up type defined in this module")
        } else {
            let dt = self
                .env
                .datatype_by_id(id)
                .expect("looking up identifier external to this module");
            (dt.mem_size(), dt.mem_align())
        };
        assert!(size > 0);
        assert!(align > 0);
        (size, align)
    }
}

struct FinalizedTypes {
    types: SecondaryMap<DatatypeIx, Option<DatatypeRepr>>,
}

impl FinalizedTypes {
    fn new(map_size: usize) -> Self {
        let mut types = SecondaryMap::new();
        types.resize(map_size);
        Self { types }
    }

    fn is_defined(&self, ix: DatatypeIx) -> bool {
        self.types.get(ix).expect("index exists in types").is_some()
    }

    fn size_align(&self, ix: DatatypeIx) -> Option<(usize, usize)> {
        if let Some(d) = self.types.get(ix).expect("index exists in types") {
            Some((d.mem_size, d.mem_align))
        } else {
            None
        }
    }

    fn define(&mut self, ix: DatatypeIx, repr: DatatypeRepr) {
        self.types[ix] = Some(repr)
    }

    fn build(self) -> PrimaryMap<DatatypeIx, DatatypeRepr> {
        let mut datatypes = PrimaryMap::new();
        for dt in self.types.values() {
            datatypes.push(dt.clone().expect("all datatypes finalized"));
        }
        datatypes
    }
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
