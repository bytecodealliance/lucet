use heck::CamelCase;
use lucet_idl::{
    AliasDataType, AtomType, BindingRef, DataTypeRef, DataTypeVariant, EnumDataType, FuncBinding,
    FuncDecl, Module, Named, StructDataType, StructMember,
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
    pub fn strat(enum_datatype: &Named<EnumDataType>) -> impl Strategy<Value = Self> {
        let name = enum_datatype.name.name.clone();
        prop::sample::select(enum_datatype.entity.members.clone()).prop_map(move |mem| EnumVal {
            enum_name: name.clone(),
            member_name: mem.name,
        })
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
    pub fn render_rustval(&self) -> String {
        self.value.render_rustval()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataTypeVal {
    Enum(EnumVal),
    Struct(StructVal),
    Alias(AliasVal),
    Atom(AtomVal),
}

impl DataTypeVal {
    pub fn render_rustval(&self) -> String {
        match self {
            DataTypeVal::Enum(a) => a.render_rustval(),
            DataTypeVal::Struct(a) => a.render_rustval(),
            DataTypeVal::Alias(a) => a.render_rustval(),
            DataTypeVal::Atom(a) => a.render_rustval(),
        }
    }
}

pub trait ModuleExt {
    fn datatype_strat(&self, dtref: &DataTypeRef) -> BoxedStrategy<DataTypeVal>;
    fn function_strat(&self) -> BoxedStrategy<FuncDecl>;
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

    fn function_strat(&self) -> BoxedStrategy<FuncDecl> {
        let decls = self.funcs.values().cloned().collect::<Vec<FuncDecl>>();
        prop::sample::select(decls).boxed()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindingVal {
    name: String,
    mutable: bool,
    variant: BindingValVariant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingValVariant {
    Value(DataTypeVal),
    Ptr(DataTypeVal),
    Array(Vec<DataTypeVal>),
}

impl BindingVal {
    fn binding_strat(module: &Module, binding: &FuncBinding, mutable: bool) -> BoxedStrategy<Self> {
        let name = binding.name.clone();
        match binding.from {
            BindingRef::Value(_) => module
                .datatype_strat(&binding.type_)
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Value(v),
                })
                .boxed(),
            BindingRef::Ptr(_) => module
                .datatype_strat(&binding.type_)
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Ptr(v),
                })
                .boxed(),
            BindingRef::Slice(_, _) => {
                prop::collection::vec(module.datatype_strat(&binding.type_), 100)
                    .prop_map(move |v| BindingVal {
                        name: name.clone(),
                        mutable,
                        variant: BindingValVariant::Array(v),
                    })
                    .boxed()
            }
        }
    }
    fn render_rust_binding(&self) -> String {
        format!(
            "let {}{} = {};",
            if self.mutable { "mut " } else { "" },
            self.name,
            self.render_rust_constructor(),
        )
    }

    fn render_rust_constructor(&self) -> String {
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

    fn render_rust_ref(&self) -> String {
        match &self.variant {
            BindingValVariant::Value(v) => v.render_rustval(),
            BindingValVariant::Ptr(v) => format!("&{}", v.render_rustval()),
            BindingValVariant::Array(vs) => format!(
                "&[{}]",
                vs.iter()
                    .map(|v| v.render_rustval())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncCallPredicate {
    func: FuncDecl,
    pre: Vec<BindingVal>,
    post: Vec<BindingVal>,
}

impl FuncCallPredicate {
    pub fn strat(module: &Module, func: &FuncDecl) -> BoxedStrategy<FuncCallPredicate> {
        let mut pre_strat: Vec<BoxedStrategy<BindingVal>> = func
            .in_bindings
            .iter()
            .map(|binding| BindingVal::binding_strat(module, binding, false))
            .collect();

        pre_strat.append(
            &mut func
                .inout_bindings
                .iter()
                .map(|binding| BindingVal::binding_strat(module, binding, true))
                .collect(),
        );
        let post_strat: Vec<BoxedStrategy<BindingVal>> = func
            .inout_bindings
            .iter()
            .chain(func.out_bindings.iter())
            .map(|binding| BindingVal::binding_strat(module, binding, false))
            .collect();

        let func = func.clone();
        (pre_strat, post_strat)
            .prop_map(move |(pre, post)| FuncCallPredicate {
                pre,
                post,
                func: func.clone(),
            })
            .boxed()
    }

    pub fn render_caller(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .pre
            .iter()
            .map(|val| val.render_rust_binding())
            .collect();

        let mut arg_syntax = Vec::new();
        for in_binding in self.func.in_bindings.iter() {
            arg_syntax.push(match in_binding.from {
                BindingRef::Ptr(_) => format!("&{}", in_binding.name),
                BindingRef::Slice(_, _) => format!("&{}", in_binding.name),
                BindingRef::Value(_) => in_binding.name.clone(),
            })
        }
        for io_binding in self.func.inout_bindings.iter() {
            arg_syntax.push(match io_binding.from {
                BindingRef::Ptr(_) => format!("&mut {}", io_binding.name),
                BindingRef::Slice(_, _) => format!("&mut {}", io_binding.name),
                BindingRef::Value(_) => unreachable!("should be no such thing as an io value"),
            })
        }

        lines.push(format!(
            "let {} = {}({});",
            render_tuple(
                self.func
                    .out_bindings
                    .iter()
                    .map(|b| b.name.clone())
                    .collect::<Vec<String>>(),
                "_"
            ),
            self.func.field_name,
            arg_syntax.join(",")
        ));
        lines.append(
            &mut self
                .post
                .iter()
                .map(|val| format!("assert_eq!({}, {});", val.name, val.render_rust_ref()))
                .collect(),
        );
        lines
    }

    pub fn render_postcondition_bindings(&self) -> Vec<String> {
        self.post
            .iter()
            .map(|val| format!("let {} = {};", val.name, val.render_rust_constructor()))
            .collect()
    }
    pub fn render_postcondition_assertions(&self) -> Vec<String> {
        self.post
            .iter()
            .map(|val| format!("assert_eq!({}, {});", val.name, val.render_rust_ref()))
            .collect()
    }
}

fn render_tuple(vs: Vec<String>, base_case: &str) -> String {
    match vs.len() {
        0 => base_case.to_owned(),
        1 => vs[0].clone(),
        _ => format!("({})", vs.join(", ")),
    }
}
