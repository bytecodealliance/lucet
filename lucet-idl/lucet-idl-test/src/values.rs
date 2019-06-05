use lucet_idl::{
    AliasDataType, AtomType, DataTypeRef, DataTypeVariant, EnumDataType, Module, Named,
    StructDataType, StructMember,
};
use proptest::{self, prelude::*};

#[derive(Debug, Clone, PartialEq)]
pub enum AtomVal {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl AtomVal {
    pub fn strat(atom_type: &AtomType) -> BoxedStrategy<Self> {
        match atom_type {
            AtomType::Bool => any::<bool>().prop_map(AtomVal::Bool).boxed(),
            AtomType::U8 => any::<u8>().prop_map(AtomVal::U8).boxed(),
            AtomType::U16 => any::<u16>().prop_map(AtomVal::U16).boxed(),
            AtomType::U32 => any::<u32>().prop_map(AtomVal::U32).boxed(),
            AtomType::U64 => any::<u64>().prop_map(AtomVal::U64).boxed(),
            AtomType::I8 => any::<i8>().prop_map(AtomVal::I8).boxed(),
            AtomType::I16 => any::<i16>().prop_map(AtomVal::I16).boxed(),
            AtomType::I32 => any::<i32>().prop_map(AtomVal::I32).boxed(),
            AtomType::I64 => any::<i64>().prop_map(AtomVal::I64).boxed(),
            AtomType::F32 => any::<f32>().prop_map(AtomVal::F32).boxed(),
            AtomType::F64 => any::<f64>().prop_map(AtomVal::F64).boxed(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVal {
    pub enum_name: String,
    pub member_name: String,
}

impl EnumVal {
    pub fn strat(enum_datatype: &Named<EnumDataType>) -> impl Strategy<Value = Self> {
        let name = enum_datatype.name.name.clone();
        proptest::sample::select(enum_datatype.entity.members.clone()).prop_map(move |mem| {
            EnumVal {
                enum_name: name.clone(),
                member_name: mem.name,
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructVal {
    pub struct_name: String,
    pub members: Vec<StructMemberVal>,
}

impl StructVal {
    pub fn strat(struct_dt: &Named<StructDataType>, module: &Module) -> BoxedStrategy<Self> {
        let name = struct_dt.name.name.clone();
        let member_strats: Vec<BoxedStrategy<StructMemberVal>> = struct_dt
            .entity
            .members
            .iter()
            .map(|m| StructMemberVal::strat(m, module))
            .collect();
        member_strats
            .prop_map(move |members| StructVal {
                struct_name: name.clone(),
                members,
            })
            .boxed()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructMemberVal {
    pub name: String,
    pub value: Box<DataTypeVal>,
}

impl StructMemberVal {
    pub fn strat(struct_member: &StructMember, module: &Module) -> BoxedStrategy<Self> {
        let name = struct_member.name.clone();
        module
            .datatype_strat(&struct_member.type_)
            .prop_map(move |value| StructMemberVal {
                name: name.clone(),
                value: Box::new(value),
            })
            .boxed()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasVal {
    pub name: String,
    pub value: Box<DataTypeVal>,
}

impl AliasVal {
    pub fn strat(alias_dt: &Named<AliasDataType>, module: &Module) -> BoxedStrategy<Self> {
        let name = alias_dt.name.name.clone();
        module
            .datatype_strat(&alias_dt.entity.to)
            .prop_map(move |value| AliasVal {
                name: name.clone(),
                value: Box::new(value),
            })
            .boxed()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataTypeVal {
    Enum(EnumVal),
    Struct(StructVal),
    Alias(AliasVal),
    Atom(AtomVal),
}

pub trait ModuleExt {
    fn datatype_strat(&self, dtref: &DataTypeRef) -> BoxedStrategy<DataTypeVal>;
}

impl ModuleExt for Module {
    fn datatype_strat(&self, dtref: &DataTypeRef) -> BoxedStrategy<DataTypeVal> {
        match dtref {
            DataTypeRef::Defined(ident) => {
                let dt = self.get_datatype(*ident).expect("ref to defined datatype");
                match dt.entity.variant {
                    DataTypeVariant::Struct(ref struct_dt) => {
                        StructVal::strat(&dt.using_name(struct_dt), self)
                            .prop_map(DataTypeVal::Struct)
                            .boxed()
                    }
                    DataTypeVariant::Enum(ref enum_dt) => EnumVal::strat(&dt.using_name(enum_dt))
                        .prop_map(DataTypeVal::Enum)
                        .boxed(),
                    DataTypeVariant::Alias(ref alias_dt) => {
                        AliasVal::strat(&dt.using_name(alias_dt), self)
                            .prop_map(DataTypeVal::Alias)
                            .boxed()
                    }
                }
            }
            DataTypeRef::Atom(a) => AtomVal::strat(&a).prop_map(DataTypeVal::Atom).boxed(),
        }
    }
}
