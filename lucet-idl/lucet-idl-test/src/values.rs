use lucet_idl::{
    AliasDataType, AtomType, DataTypeRef, DataTypeVariant, EnumDataType, Module, StructDataType,
    StructMember,
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
    pub member_name: String,
}

impl EnumVal {
    pub fn strat(enum_datatype: &EnumDataType) -> impl Strategy<Value = Self> {
        proptest::sample::select(enum_datatype.members.clone()).prop_map(|mem| EnumVal {
            member_name: mem.name,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructVal {
    pub members: Vec<StructMemberVal>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructMemberVal {
    pub name: String,
    pub value: Box<DataTypeVal>,
}

impl StructMemberVal {
    pub fn strat(struct_member: &StructMember, module: &Module) -> BoxedStrategy<Self> {
        module
            .datatype_strat(&struct_member.type_)
            .prop_map(|value| StructMemberVal {
                name: struct_member.name.clone(),
                value: Box::new(value),
            })
            .boxed()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasVal {}

#[derive(Debug, Clone, PartialEq)]
pub enum DataTypeVal {
    Enum(EnumVal),
    Struct(StructVal),
    Alias(AliasVal),
    Atom(AtomVal),
}

trait ModuleExt {
    fn datatype_strat(&self, dtref: &DataTypeRef) -> BoxedStrategy<DataTypeVal>;
}

impl ModuleExt for Module {
    fn datatype_strat(&self, dtref: &DataTypeRef) -> BoxedStrategy<DataTypeVal> {
        match dtref {
            DataTypeRef::Defined(ident) => {
                let dt = self.get_datatype(*ident).expect("ref to defined datatype");
                match dt.entity.variant {
                    DataTypeVariant::Struct(struct_dt) => unimplemented!(),
                    DataTypeVariant::Enum(enum_dt) => unimplemented!(),
                    DataTypeVariant::Alias(alias_dt) => unimplemented!(),
                }
            }
            DataTypeRef::Atom(a) => AtomVal::strat(&a).prop_map(DataTypeVal::Atom).boxed(),
        }
    }
}
