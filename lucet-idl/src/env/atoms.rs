use crate::env::MemArea;

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

impl MemArea for AtomType {
    fn mem_size(&self) -> usize {
        match self {
            AtomType::Bool => 1,
            AtomType::U8 | AtomType::I8 => 1,
            AtomType::U16 | AtomType::I16 => 2,
            AtomType::U32 | AtomType::I32 | AtomType::F32 => 4,
            AtomType::U64 | AtomType::I64 | AtomType::F64 => 8,
        }
    }
    fn mem_align(&self) -> usize {
        self.mem_size()
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

impl MemArea for AbiType {
    fn mem_size(&self) -> usize {
        match self {
            AbiType::I32 | AbiType::F32 => 4,
            AbiType::I64 | AbiType::F64 => 8,
        }
    }

    fn mem_align(&self) -> usize {
        self.mem_size()
    }
}
