use heck::CamelCase;
use lucet_idl::{
    AliasDatatype, AtomType, BindingDirection, BindingParam, Datatype, DatatypeVariant,
    EnumDatatype, RustIdiomArg, RustIdiomRet, StructDatatype, StructMember,
};
use proptest::prelude::*;

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
    pub fn trivial(atom_type: &AtomType) -> Self {
        match atom_type {
            AtomType::Bool => AtomVal::Bool(false),
            AtomType::U8 => AtomVal::U8(0),
            AtomType::U16 => AtomVal::U16(0),
            AtomType::U32 => AtomVal::U32(0),
            AtomType::U64 => AtomVal::U64(0),
            AtomType::I8 => AtomVal::I8(0),
            AtomType::I16 => AtomVal::I16(0),
            AtomType::I32 => AtomVal::I32(0),
            AtomType::I64 => AtomVal::I64(0),
            AtomType::F32 => AtomVal::F32(0.0),
            AtomType::F64 => AtomVal::F64(0.0),
        }
    }
    pub fn render_rustval(&self) -> String {
        match self {
            AtomVal::Bool(v) => format!("{}", v),
            AtomVal::U8(v) => format!("{}", v),
            AtomVal::U16(v) => format!("{}", v),
            AtomVal::U32(v) => format!("{}", v),
            AtomVal::U64(v) => format!("{}", v),
            AtomVal::I8(v) => format!("{}", v),
            AtomVal::I16(v) => format!("{}", v),
            AtomVal::I32(v) => format!("{}", v),
            AtomVal::I64(v) => format!("{}", v),
            AtomVal::F32(v) => format!("{}f32", v),
            AtomVal::F64(v) => format!("{}f64", v),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVal {
    pub enum_name: String,
    pub member_name: String,
}

impl EnumVal {
    pub fn strat(enum_datatype: &EnumDatatype) -> impl Strategy<Value = Self> {
        let name = enum_datatype.datatype().name().to_owned();
        prop::sample::select(
            enum_datatype
                .variants()
                .map(|v| v.name().to_owned())
                .collect::<Vec<String>>(),
        )
        .prop_map(move |mem_name| EnumVal {
            enum_name: name.clone(),
            member_name: mem_name.clone(),
        })
    }
    pub fn trivial(enum_datatype: &EnumDatatype) -> Self {
        let enum_name = enum_datatype.datatype().name().to_owned();
        let member_name = enum_datatype
            .variants()
            .collect::<Vec<_>>()
            .get(0)
            .expect("at least one variant")
            .name()
            .to_owned();
        EnumVal {
            enum_name,
            member_name,
        }
    }
    pub fn render_rustval(&self) -> String {
        format!(
            "{}::{}",
            self.enum_name.to_camel_case(),
            self.member_name.to_camel_case()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructVal {
    pub struct_name: String,
    pub members: Vec<StructMemberVal>,
}

impl StructVal {
    pub fn strat(struct_dt: &StructDatatype) -> BoxedStrategy<Self> {
        let name = struct_dt.datatype().name().to_owned();
        let member_strats: Vec<BoxedStrategy<StructMemberVal>> = struct_dt
            .members()
            .map(|m| StructMemberVal::strat(&m))
            .collect();
        member_strats
            .prop_map(move |members| StructVal {
                struct_name: name.clone(),
                members,
            })
            .boxed()
    }
    pub fn trivial(struct_dt: &StructDatatype) -> Self {
        let struct_name = struct_dt.datatype().name().to_owned();
        let members = struct_dt
            .members()
            .map(|m| StructMemberVal::trivial(&m))
            .collect();
        StructVal {
            struct_name,
            members,
        }
    }
    pub fn render_rustval(&self) -> String {
        let members = self
            .members
            .iter()
            .map(|v| format!("{}: {}", v.name, v.value.render_rustval()))
            .collect::<Vec<String>>();
        format!(
            "{} {{ {} }}",
            self.struct_name.to_camel_case(),
            members.join(", ")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructMemberVal {
    pub name: String,
    pub value: Box<DatatypeVal>,
}

impl StructMemberVal {
    pub fn strat(struct_member: &StructMember) -> BoxedStrategy<Self> {
        let name = struct_member.name().to_owned();
        struct_member
            .type_()
            .strat()
            .prop_map(move |value| StructMemberVal {
                name: name.clone(),
                value: Box::new(value),
            })
            .boxed()
    }
    pub fn trivial(struct_member: &StructMember) -> Self {
        let name = struct_member.name().to_owned();
        let value = Box::new(struct_member.type_().trivial_val());
        StructMemberVal { name, value }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasVal {
    pub name: String,
    pub value: Box<DatatypeVal>,
}

impl AliasVal {
    pub fn strat(alias_dt: &AliasDatatype) -> BoxedStrategy<Self> {
        let name = alias_dt.datatype().name().to_owned();
        alias_dt
            .to()
            .strat()
            .prop_map(move |value| AliasVal {
                name: name.clone(),
                value: Box::new(value),
            })
            .boxed()
    }
    pub fn trivial(alias_dt: &AliasDatatype) -> Self {
        let name = alias_dt.datatype().name().to_owned();
        let value = Box::new(alias_dt.to().trivial_val());
        AliasVal { name, value }
    }
    pub fn render_rustval(&self) -> String {
        self.value.render_rustval()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatatypeVal {
    Enum(EnumVal),
    Struct(StructVal),
    Alias(AliasVal),
    Atom(AtomVal),
}

impl DatatypeVal {
    pub fn render_rustval(&self) -> String {
        match self {
            DatatypeVal::Enum(a) => a.render_rustval(),
            DatatypeVal::Struct(a) => a.render_rustval(),
            DatatypeVal::Alias(a) => a.render_rustval(),
            DatatypeVal::Atom(a) => a.render_rustval(),
        }
    }
}

pub trait DatatypeExt {
    fn strat(&self) -> BoxedStrategy<DatatypeVal>;
    fn trivial_val(&self) -> DatatypeVal;
}

impl<'a> DatatypeExt for Datatype<'a> {
    fn strat(&self) -> BoxedStrategy<DatatypeVal> {
        match self.variant() {
            DatatypeVariant::Struct(struct_dt) => StructVal::strat(&struct_dt)
                .prop_map(DatatypeVal::Struct)
                .boxed(),
            DatatypeVariant::Enum(enum_dt) => {
                EnumVal::strat(&enum_dt).prop_map(DatatypeVal::Enum).boxed()
            }
            DatatypeVariant::Alias(alias_dt) => AliasVal::strat(&alias_dt)
                .prop_map(DatatypeVal::Alias)
                .boxed(),
            DatatypeVariant::Atom(a) => AtomVal::strat(&a).prop_map(DatatypeVal::Atom).boxed(),
        }
    }
    fn trivial_val(&self) -> DatatypeVal {
        match self.variant() {
            DatatypeVariant::Struct(struct_dt) => {
                DatatypeVal::Struct(StructVal::trivial(&struct_dt))
            }
            DatatypeVariant::Enum(enum_dt) => DatatypeVal::Enum(EnumVal::trivial(&enum_dt)),
            DatatypeVariant::Alias(alias_dt) => DatatypeVal::Alias(AliasVal::trivial(&alias_dt)),
            DatatypeVariant::Atom(atom_dt) => DatatypeVal::Atom(AtomVal::trivial(&atom_dt)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingVal {
    pub name: String,
    pub mutable: bool,
    pub variant: BindingValVariant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingValVariant {
    Value(DatatypeVal),
    Ptr(DatatypeVal),
    Array(Vec<DatatypeVal>),
}

impl BindingVal {
    pub fn arg_strat(arg: &RustIdiomArg) -> BoxedStrategy<Self> {
        let mutable = arg.direction() == BindingDirection::InOut;
        let name = arg.name();
        match arg.param() {
            BindingParam::Value(_) => arg
                .type_()
                .strat()
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Value(v),
                })
                .boxed(),
            BindingParam::Ptr(_) => arg
                .type_()
                .strat()
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Ptr(v),
                })
                .boxed(),
            BindingParam::Slice(_, _) => prop::collection::vec(arg.type_().strat(), 100)
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Array(v),
                })
                .boxed(),
        }
    }

    pub fn arg_trivial(arg: &RustIdiomArg) -> Self {
        let name = arg.name();
        let mutable = arg.direction() == BindingDirection::InOut;
        let trivial_val = arg.type_().trivial_val();
        BindingVal {
            name,
            mutable,
            variant: match arg.param() {
                BindingParam::Value(_) => BindingValVariant::Value(trivial_val),
                BindingParam::Ptr(_) => BindingValVariant::Ptr(trivial_val),
                BindingParam::Slice(_, _) => BindingValVariant::Array(vec![trivial_val]),
            },
        }
    }
    pub fn ret_strat(ret: &RustIdiomRet) -> BoxedStrategy<Self> {
        let name = ret.name();
        // There can only be param or value bindings on returns,
        // and both are idiomatically values.
        ret.type_()
            .strat()
            .prop_map(move |v| BindingVal {
                name: name.clone(),
                mutable: false,
                variant: BindingValVariant::Value(v),
            })
            .boxed()
    }
    pub fn ret_trivial(ret: &RustIdiomRet) -> Self {
        let name = ret.name();
        let trivial_val = ret.type_().trivial_val();
        BindingVal {
            name,
            mutable: false,
            variant: BindingValVariant::Value(trivial_val),
        }
    }

    pub fn render_rust_binding(&self) -> String {
        format!(
            "let {}{} = {};",
            if self.mutable { "mut " } else { "" },
            self.name,
            self.render_rust_constructor(),
        )
    }

    pub fn render_rust_constructor(&self) -> String {
        match &self.variant {
            BindingValVariant::Value(v) => v.render_rustval(),
            BindingValVariant::Ptr(v) => v.render_rustval(),
            BindingValVariant::Array(vs) => format!(
                "vec![{}]",
                vs.iter()
                    .map(|v| v.render_rustval())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }

    pub fn render_rust_ref(&self) -> String {
        match &self.variant {
            BindingValVariant::Value(v) => v.render_rustval(),
            BindingValVariant::Ptr(v) => format!("&{}", v.render_rustval()),
            BindingValVariant::Array(vs) => format!(
                "vec![{}].as_slice()",
                vs.iter()
                    .map(|v| v.render_rustval())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

pub fn render_tuple(vs: &[String], base_case: &str) -> String {
    match vs.len() {
        0 => base_case.to_owned(),
        1 => vs[0].clone(),
        _ => format!("({})", vs.join(", ")),
    }
}
