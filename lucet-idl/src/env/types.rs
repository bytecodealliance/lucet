use crate::datatypes::{AbiType, AtomType};

use cranelift_entity::{entity_impl, PrimaryMap};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ModuleIx(u32);
entity_impl!(ModuleIx);

#[derive(Debug, Clone)]
struct PackageRepr {
    pub names: Vec<String>,
    pub modules: PrimaryMap<ModuleIx, ModuleRepr>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DatatypeIx(u32);
entity_impl!(DatatypeIx);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatatypeIdent(ModuleIx, DatatypeIx);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FuncIx(u32);
entity_impl!(FuncIx);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncIdent(ModuleIx, FuncIx);

#[derive(Debug, Clone)]
pub struct ModuleRepr {
    pub datatype_names: Vec<String>,
    pub datatypes: PrimaryMap<DatatypeIx, DatatypeRepr>,
    pub func_names: Vec<String>,
    pub funcs: PrimaryMap<FuncIx, FuncRepr>,
}

#[derive(Debug, Clone)]
pub struct DatatypeRepr {
    pub variant: DatatypeVariant,
    pub repr_size: usize,
    pub align: usize,
}

#[derive(Debug, Clone)]
pub enum DatatypeVariant {
    Atom(AtomType),
    Struct(StructDatatype),
    Enum(EnumDatatype),
    Alias(AliasDatatype),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMember {
    pub type_: DatatypeIdent,
    pub name: String,
    pub offset: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructDatatype {
    pub members: Vec<StructMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumMember {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumDatatype {
    pub members: Vec<EnumMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AliasDatatype {
    pub to: DatatypeIdent,
}

#[derive(Debug, Clone)]
pub struct FuncRepr {
    pub args: PrimaryMap<ArgIx, ParamRepr>,
    pub rets: PrimaryMap<RetIx, ParamRepr>,
    pub bindings: PrimaryMap<BindingIx, BindingRepr>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ArgIx(u32);
entity_impl!(ArgIx);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RetIx(u32);
entity_impl!(RetIx);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ParamIx {
    Arg(ArgIx),
    Ret(RetIx),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BindingIx(u32);
entity_impl!(BindingIx);

#[derive(Debug, Clone)]
pub struct ParamRepr {
    pub name: String,
    pub type_: AbiType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingRepr {
    pub name: String,
    pub type_: DatatypeIdent,
    pub direction: BindingDirection,
    pub from: BindingFromRepr,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BindingDirection {
    In,
    InOut,
    Out,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BindingFromRepr {
    Ptr(ParamIx),
    Slice(ParamIx, ParamIx),
    Value(ParamIx),
}
