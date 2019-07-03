use std::fmt;

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

impl AtomType {
    pub fn repr_size(&self) -> usize {
        match self {
            AtomType::Bool => 1,
            AtomType::U8 | AtomType::I8 => 1,
            AtomType::U16 | AtomType::I16 => 2,
            AtomType::U32 | AtomType::I32 | AtomType::F32 => 4,
            AtomType::U64 | AtomType::I64 | AtomType::F64 => 8,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AbiType {
    I32,
    I64,
    F32,
    F64,
}

impl AbiType {
    pub fn repr_size(&self) -> usize {
        match self {
            AbiType::I32 | AbiType::F32 => 4,
            AbiType::I64 | AbiType::F64 => 8,
        }
    }

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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Ident(pub usize);

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DataTypeRef {
    Defined(Ident),
    Atom(AtomType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMember {
    pub type_: DataTypeRef,
    pub name: String,
    pub offset: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructDataType {
    pub members: Vec<StructMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumMember {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumDataType {
    pub members: Vec<EnumMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AliasDataType {
    pub to: DataTypeRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DataTypeVariant {
    Struct(StructDataType),
    Enum(EnumDataType),
    Alias(AliasDataType),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataType {
    pub variant: DataTypeVariant,
    pub repr_size: usize,
    pub align: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncArg {
    pub name: String,
    pub type_: AbiType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncDecl {
    pub field_name: String,
    pub binding_name: String,
    pub args: Vec<FuncArg>,
    pub rets: Vec<FuncArg>,
    pub bindings: Vec<FuncBinding>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncBinding {
    pub name: String,
    pub type_: DataTypeRef,
    pub direction: BindDirection,
    pub from: BindingRef,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BindingRef {
    Ptr(String),           // Treat the argument of that name as a pointer
    Slice(String, String), // Treat first argument as a pointer, second as the length
    Value(String),         // Treat the argument of that name as a value
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BindDirection {
    In,
    Out,
    InOut,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Name {
    pub name: String,
    pub location: Location,
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// A convenient structure holding a data type, its name and
/// its internal IDL representation
#[derive(Debug, Clone)]
pub struct Named<'t, E> {
    pub id: Ident,
    pub name: &'t Name,
    pub entity: &'t E,
}

impl<'a, T> Named<'a, T> {
    pub fn using_name<U>(&self, other: &'a U) -> Named<'a, U> {
        Named {
            id: self.id,
            name: self.name,
            entity: other,
        }
    }
}

impl<'a> Named<'a, DataType> {
    pub fn datatype_ref(&self) -> DataTypeRef {
        DataTypeRef::Defined(self.id)
    }
}
