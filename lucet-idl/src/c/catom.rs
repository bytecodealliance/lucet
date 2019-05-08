use crate::target::*;
use crate::types::*;

/// Information about a C native type
pub struct CAtom {
    pub native_type_name: &'static str,
    pub native_type_size: usize,
    pub native_type_align: usize,
}

impl From<AtomType> for CAtom {
    fn from(atom_type: AtomType) -> Self {
        match atom_type {
            AtomType::Bool => CAtom {
                native_type_name: "bool",
                native_type_size: 1,
                native_type_align: 1,
            },
            AtomType::U8 => CAtom {
                native_type_name: "uint8_t",
                native_type_size: 1,
                native_type_align: 1,
            },
            AtomType::U16 => CAtom {
                native_type_name: "uint16_t",
                native_type_size: 2,
                native_type_align: 2,
            },
            AtomType::U32 => CAtom {
                native_type_name: "uint32_t",
                native_type_size: 4,
                native_type_align: 4,
            },
            AtomType::U64 => CAtom {
                native_type_name: "uint64_t",
                native_type_size: 8,
                native_type_align: 8, // x86_64 alignment rule
            },
            AtomType::I8 => CAtom {
                native_type_name: "int8_t",
                native_type_size: 1,
                native_type_align: 1,
            },
            AtomType::I16 => CAtom {
                native_type_name: "int16_t",
                native_type_size: 2,
                native_type_align: 2,
            },
            AtomType::I32 => CAtom {
                native_type_name: "int32_t",
                native_type_size: 4,
                native_type_align: 4,
            },
            AtomType::I64 => CAtom {
                native_type_name: "int64_t",
                native_type_size: 8,
                native_type_align: 8, // x86_64 alignment rule
            },
            AtomType::F32 => CAtom {
                native_type_name: "float",
                native_type_size: 4,
                native_type_align: 4,
            },
            AtomType::F64 => CAtom {
                native_type_name: "double",
                native_type_size: 8,
                native_type_align: 8, // x86_64 alignment rule
            },
        }
    }
}

impl CAtom {
    /// Native type used for enums
    /// X86_64 ABI says its an U32, wasm32 seems to agree
    pub fn enum_() -> Self {
        CAtom::from(AtomType::U32)
    }

    /// C atom type to generic atom type
    pub fn as_atom_type(&self) -> Option<AtomType> {
        match self.native_type_name {
            "uint8_t" => Some(AtomType::U8),
            "uint16_t" => Some(AtomType::U16),
            "uint32_t" => Some(AtomType::U32),
            "uint64_t" => Some(AtomType::U64),
            "int8_t" => Some(AtomType::I8),
            "int16_t" => Some(AtomType::I16),
            "int32_t" => Some(AtomType::I32),
            "int64_t" => Some(AtomType::I64),
            "float" => Some(AtomType::F32),
            "double" => Some(AtomType::F64),
            _ => None,
        }
    }

    /// Return an expression to swap byte order
    /// No need to swap anything if we have a single byte or if we explicitly
    /// target an architecture that matches the reference target for the type
    pub fn bswap(&self, _target: Target, vstr: &str) -> String {
        if self.native_type_size < 2 {
            vstr.to_string()
        } else {
            format!("___bswap_{}({})", self.native_type_name, vstr)
        }
    }

    /// Return an expression to force little-endian order
    pub fn little_endian(&self, target: Target, vstr: &str) -> String {
        let atom_type = self.as_atom_type();
        if self.native_type_size < 2
            || (atom_type.is_some() && target.uses_reference_target_endianness())
        {
            vstr.to_string()
        } else {
            format!("___le_{}({})", self.native_type_name, vstr)
        }
    }
}
