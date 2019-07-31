pub use crate::env::atoms::{AbiType, AtomType};
pub use crate::env::MemArea;

use std::fmt;

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
pub struct Name {
    pub name: String,
    pub location: Location,
}

impl MemArea for DataType {
    fn mem_size(&self) -> usize {
        self.repr_size
    }
    fn mem_align(&self) -> usize {
        self.align
    }
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
