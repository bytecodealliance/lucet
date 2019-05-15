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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Attr {
    pub key: String,
    pub val: String,
    pub location: Location,
}

impl Attr {
    pub fn new(key: &str, val: &str, location: Location) -> Attr {
        Attr {
            key: key.to_owned(),
            val: val.to_owned(),
            location,
        }
    }
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
    pub attrs: Vec<Attr>,
    pub repr_size: usize,
    pub offset: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructDataType {
    pub members: Vec<StructMember>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumMember {
    pub name: String,
    pub attrs: Vec<Attr>,
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
    pub attrs: Vec<Attr>,
    pub repr_size: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncArg {
    pub type_: DataTypeRef,
    pub name: String,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncDecl {
    pub args: Vec<FuncArg>,
    pub rets: Vec<FuncRet>,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncRet {
    pub type_: DataTypeRef,
    pub attrs: Vec<Attr>,
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
