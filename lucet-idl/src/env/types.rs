use cranelift_entity::{entity_impl, PrimaryMap};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ModuleIx(u32);
entity_impl!(ModuleIx);

#[derive(Debug, Clone)]
pub struct PackageRepr {
    pub names: PrimaryMap<ModuleIx, String>,
    pub modules: PrimaryMap<ModuleIx, ModuleRepr>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DatatypeIx(u32);
entity_impl!(DatatypeIx);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DatatypeIdent {
    pub module: ModuleIx,
    pub datatype: DatatypeIx,
}

impl DatatypeIdent {
    pub fn new(module: ModuleIx, datatype: DatatypeIx) -> Self {
        DatatypeIdent { module, datatype }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FuncIx(u32);
entity_impl!(FuncIx);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncIdent(ModuleIx, FuncIx);

#[derive(Debug, Clone)]
pub struct ModuleRepr {
    pub datatypes: ModuleDatatypesRepr,
    pub funcs: ModuleFuncsRepr,
}

#[derive(Debug, Clone)]
pub struct ModuleDatatypesRepr {
    pub names: PrimaryMap<DatatypeIx, String>,
    pub datatypes: PrimaryMap<DatatypeIx, DatatypeRepr>,
}

#[derive(Debug, Clone)]
pub struct ModuleFuncsRepr {
    pub names: PrimaryMap<FuncIx, String>,
    pub funcs: PrimaryMap<FuncIx, FuncRepr>,
}

#[derive(Debug, Clone)]
pub struct DatatypeRepr {
    pub variant: DatatypeVariantRepr,
    pub repr_size: usize,
    pub align: usize,
}

#[derive(Debug, Clone)]
pub enum DatatypeVariantRepr {
    Atom(AtomType),
    Struct(StructDatatypeRepr),
    Enum(EnumDatatypeRepr),
    Alias(AliasDatatypeRepr),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AtomType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AbiType {
    I32,
    I64,
    F32,
    F64,
}

impl AbiType {
    pub fn from_atom(a: &AtomType) -> Self {
        match a {
            AtomType::Bool
            | AtomType::U8
            | AtomType::I8
            | AtomType::U16
            | AtomType::I16
            | AtomType::U32
            | AtomType::I32 => AbiType::I32,
            AtomType::I64 | AtomType::U64 => AbiType::I64,
            AtomType::F32 => AbiType::F32,
            AtomType::F64 => AbiType::F64,
        }
    }

    pub fn of_atom(a: AtomType) -> Option<Self> {
        match a {
            AtomType::I32 => Some(AbiType::I32),
            AtomType::I64 => Some(AbiType::I64),
            AtomType::F32 => Some(AbiType::F32),
            AtomType::F64 => Some(AbiType::F64),
            _ => None,
        }
    }
}

impl From<AbiType> for AtomType {
    fn from(abi: AbiType) -> AtomType {
        match abi {
            AbiType::I32 => AtomType::I32,
            AbiType::I64 => AtomType::I64,
            AbiType::F32 => AtomType::F32,
            AbiType::F64 => AtomType::F64,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMemberRepr {
    pub type_: DatatypeIdent,
    pub name: String,
    pub offset: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructDatatypeRepr {
    pub members: Vec<StructMemberRepr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumMember {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumDatatypeRepr {
    pub members: Vec<EnumMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AliasDatatypeRepr {
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
