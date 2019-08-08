use heck::{CamelCase, SnakeCase};
use lucet_idl::{
    pretty_writer::PrettyWriter, AliasDatatype, AtomType, BindingDirection, BindingParam, Datatype,
    DatatypeVariant, EnumDatatype, FuncBinding, Function, Module, RustFunc, RustName, RustTypeName,
    StructDatatype, StructMember,
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
    pub fn strat(enum_datatype: &EnumDatatype) -> impl Strategy<Value = Self> {
        let name = enum_datatype.name().to_owned();
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
        let name = struct_dt.name().to_owned();
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct AliasVal {
    pub name: String,
    pub value: Box<DatatypeVal>,
}

impl AliasVal {
    pub fn strat(alias_dt: &AliasDatatype) -> BoxedStrategy<Self> {
        let name = alias_dt.name().to_owned();
        alias_dt
            .to()
            .strat()
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
}

impl<'a> DatatypeExt for Datatype<'a> {
    fn strat(&self) -> BoxedStrategy<DatatypeVal> {
        match self.variant() {
            DatatypeVariant::Struct(ref struct_dt) => StructVal::strat(struct_dt)
                .prop_map(DatatypeVal::Struct)
                .boxed(),
            DatatypeVariant::Enum(ref enum_dt) => {
                EnumVal::strat(enum_dt).prop_map(DatatypeVal::Enum).boxed()
            }
            DatatypeVariant::Alias(ref alias_dt) => AliasVal::strat(alias_dt)
                .prop_map(DatatypeVal::Alias)
                .boxed(),
            DatatypeVariant::Atom(a) => AtomVal::strat(&a).prop_map(DatatypeVal::Atom).boxed(),
        }
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
    Value(DatatypeVal),
    Ptr(DatatypeVal),
    Array(Vec<DatatypeVal>),
}

impl BindingVal {
    fn binding_strat(binding: &FuncBinding, mutable: bool) -> BoxedStrategy<Self> {
        let name = binding.name().to_owned();
        match binding.param() {
            BindingParam::Value(_) => binding
                .type_()
                .strat()
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Value(v),
                })
                .boxed(),
            BindingParam::Ptr(_) => binding
                .type_()
                .strat()
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Ptr(v),
                })
                .boxed(),
            BindingParam::Slice(_, _) => prop::collection::vec(binding.type_().strat(), 100)
                .prop_map(move |v| BindingVal {
                    name: name.clone(),
                    mutable,
                    variant: BindingValVariant::Array(v),
                })
                .boxed(),
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
    func_name: String,
    func_call_args: Vec<String>,
    func_call_rets: Vec<String>,
    func_sig_args: Vec<String>,
    func_sig_rets: Vec<String>,
    pre: Vec<BindingVal>,
    post: Vec<BindingVal>,
}

impl FuncCallPredicate {
    pub fn strat(func: &Function) -> BoxedStrategy<FuncCallPredicate> {
        let mut pre_strat: Vec<BoxedStrategy<BindingVal>> = func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::In)
            .map(|binding| BindingVal::binding_strat(&binding, false))
            .collect();

        pre_strat.append(
            &mut func
                .bindings()
                .filter(|b| b.direction() == BindingDirection::InOut)
                .map(|binding| BindingVal::binding_strat(&binding, true))
                .collect(),
        );
        let post_strat: Vec<BoxedStrategy<BindingVal>> = func
            .bindings()
            .filter(|b| {
                b.direction() == BindingDirection::InOut || b.direction() == BindingDirection::Out
            })
            .map(|binding| BindingVal::binding_strat(&binding, false))
            .collect();

        let idiom_args = func.rust_idiom_args();
        let func_name = func.rust_name();
        let func_call_args = idiom_args.iter().map(|a| a.arg_value()).collect::<Vec<_>>();
        let func_sig_args = idiom_args
            .iter()
            .map(|a| a.arg_declaration())
            .collect::<Vec<_>>();

        let idiom_rets = func.rust_idiom_rets();
        let func_call_rets = idiom_rets.iter().map(|r| r.name()).collect::<Vec<_>>();
        let func_sig_rets = idiom_rets
            .iter()
            .map(|a| a.ret_declaration())
            .collect::<Vec<_>>();

        (pre_strat, post_strat)
            .prop_map(move |(pre, post)| FuncCallPredicate {
                func_name: func_name.clone(),
                func_call_args: func_call_args.clone(),
                func_call_rets: func_call_rets.clone(),
                func_sig_args: func_sig_args.clone(),
                func_sig_rets: func_sig_rets.clone(),
                pre,
                post,
            })
            .boxed()
    }

    pub fn render_caller(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .pre
            .iter()
            .map(|val| val.render_rust_binding())
            .collect();

        lines.push(format!(
            "let {} = {}({}).unwrap();",
            render_tuple(&self.func_call_rets, "_"),
            self.func_name,
            self.func_call_args.join(",")
        ));
        lines.append(
            &mut self
                .post
                .iter()
                .map(|val| {
                    format!(
                        "assert_eq!({}, {});",
                        val.name,
                        val.render_rust_constructor()
                    )
                })
                .collect(),
        );
        lines
    }

    pub fn render_callee(&self, w: &mut PrettyWriter) {
        w.writeln(format!(
            "fn {}(&mut self, {}) -> Result<{}, ()> {{",
            self.func_name,
            self.func_sig_args.join(", "),
            render_tuple(&self.func_sig_rets, "()")
        ))
        .indent();
        // Assert preconditions hold
        w.writelns(
            &self
                .pre
                .iter()
                .map(|val| format!("assert_eq!({}, {};", val.name, val.render_rust_ref()))
                .collect::<Vec<_>>(),
        );
        // Make postconditions hold
        w.writelns(
            &self
                .post
                .iter()
                .map(|val| format!("*{} = {};", val.name, val.render_rust_constructor()))
                .collect::<Vec<_>>(),
        );
        w.eob().writeln("}");
    }
}

#[derive(Debug, Clone)]
pub struct ModuleTestPlan {
    pub module_name: String,
    module_type_name: String,
    func_predicates: Vec<FuncCallPredicate>,
}

impl ModuleTestPlan {
    pub fn strat(module: &Module) -> BoxedStrategy<ModuleTestPlan> {
        let module_name = module.name().to_snake_case();
        let module_type_name = module.rust_type_name();
        module
            .functions()
            .map(|f| FuncCallPredicate::strat(&f))
            .collect::<Vec<_>>()
            .prop_map(move |func_predicates| ModuleTestPlan {
                module_name: module_name.clone(),
                module_type_name: module_type_name.clone(),
                func_predicates,
            })
            .boxed()
    }

    pub fn render_guest(&self, w: &mut PrettyWriter) {
        for func in self.func_predicates.iter() {
            w.writelns(&func.render_caller());
        }
    }

    pub fn render_host(&self, mut w: &mut PrettyWriter) {
        w.writeln("struct TestHarness;");
        w.writeln(format!("impl {} for TestHarness {{", self.module_type_name,))
            .indent();
        for func in self.func_predicates.iter() {
            func.render_callee(&mut w)
        }
        w.eob().writeln("}");
    }
}

fn render_tuple(vs: &[String], base_case: &str) -> String {
    match vs.len() {
        0 => base_case.to_owned(),
        1 => vs[0].clone(),
        _ => format!("({})", vs.join(", ")),
    }
}
