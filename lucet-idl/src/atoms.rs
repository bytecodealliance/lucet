use crate::MemArea;

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

const ATOM_TYPE_NAMES: &'static [(AtomType, &'static str)] = &[
    (AtomType::Bool, "bool"),
    (AtomType::U8, "u8"),
    (AtomType::U16, "u16"),
    (AtomType::U32, "u32"),
    (AtomType::U64, "u64"),
    (AtomType::I8, "i8"),
    (AtomType::I16, "i16"),
    (AtomType::I32, "i32"),
    (AtomType::I64, "i64"),
    (AtomType::F32, "f32"),
    (AtomType::F64, "f64"),
];

#[cfg(test)]
#[test]
fn atom_type_names_are_indexed_correctly() {
    for (ix, (atom, _name)) in ATOM_TYPE_NAMES.iter().enumerate() {
        assert_eq!(ix, *atom as usize);
    }
}

impl ::std::convert::TryFrom<&str> for AtomType {
    type Error = ();
    fn try_from(name: &str) -> Result<AtomType, ()> {
        ATOM_TYPE_NAMES
            .iter()
            .find(|(_a, n)| *n == name)
            .map(|(a, _n)| a.clone())
            .ok_or(())
    }
}

impl ::std::fmt::Display for AtomType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let (_a, n) = ATOM_TYPE_NAMES[*self as usize];
        write!(f, "{}", n)
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
    pub fn smallest_representation(a: &AtomType) -> Self {
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

    pub fn can_represent(&self, a: &Self) -> bool {
        match self {
            AbiType::I64 => *a == AbiType::I32 || *a == AbiType::I64,
            _ => a == self,
        }
    }

    pub fn from_atom(a: AtomType) -> Option<Self> {
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

impl ::std::convert::TryFrom<&str> for AbiType {
    type Error = ();
    fn try_from(name: &str) -> Result<AbiType, ()> {
        let atom = AtomType::try_from(name)?;
        Self::from_atom(atom).ok_or(())
    }
}

impl ::std::fmt::Display for AbiType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        write!(f, "{}", AtomType::from(self.clone()))
    }
}
