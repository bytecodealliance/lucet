use crate::env::types::{AbiType, AtomType};

pub trait MemArea {
    fn repr_size(&self) -> usize;
    fn align(&self) -> usize;
}

impl MemArea for AtomType {
    fn repr_size(&self) -> usize {
        match self {
            AtomType::Bool => 1,
            AtomType::U8 | AtomType::I8 => 1,
            AtomType::U16 | AtomType::I16 => 2,
            AtomType::U32 | AtomType::I32 | AtomType::F32 => 4,
            AtomType::U64 | AtomType::I64 | AtomType::F64 => 8,
        }
    }
    fn align(&self) -> usize {
        self.repr_size()
    }
}

impl MemArea for AbiType {
    fn repr_size(&self) -> usize {
        match self {
            AbiType::I32 | AbiType::F32 => 4,
            AbiType::I64 | AbiType::F64 => 8,
        }
    }

    fn align(&self) -> usize {
        self.repr_size()
    }
}
