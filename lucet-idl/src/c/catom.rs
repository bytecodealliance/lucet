use crate::target::*;
use crate::types::*;

/// Information about a C native type
pub struct CAtom {
    pub atom_type: AtomType,
    pub native_type_name: &'static str,
}

impl From<AtomType> for CAtom {
    fn from(atom_type: AtomType) -> Self {
        match atom_type {
            AtomType::Bool => CAtom {
                atom_type,
                native_type_name: "bool",
            },
            AtomType::U8 => CAtom {
                atom_type,
                native_type_name: "uint8_t",
            },
            AtomType::U16 => CAtom {
                atom_type,
                native_type_name: "uint16_t",
            },
            AtomType::U32 => CAtom {
                atom_type,
                native_type_name: "uint32_t",
            },
            AtomType::U64 => CAtom {
                atom_type,
                native_type_name: "uint64_t",
            },
            AtomType::I8 => CAtom {
                atom_type,
                native_type_name: "int8_t",
            },
            AtomType::I16 => CAtom {
                atom_type,
                native_type_name: "int16_t",
            },
            AtomType::I32 => CAtom {
                atom_type,
                native_type_name: "int32_t",
            },
            AtomType::I64 => CAtom {
                atom_type,
                native_type_name: "int64_t",
            },
            AtomType::F32 => CAtom {
                atom_type,
                native_type_name: "float",
            },
            AtomType::F64 => CAtom {
                atom_type,
                native_type_name: "double",
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

    /// Return an expression to swap byte order
    /// No need to swap anything if we have a single byte or if we explicitly
    /// target an architecture that matches the reference target for the type
    pub fn bswap(&self, _target: Target, vstr: &str) -> String {
        if self.atom_type.repr_size() < 2 {
            vstr.to_string()
        } else {
            format!("___bswap_{}({})", self.native_type_name, vstr)
        }
    }

    /// Return an expression to force little-endian order
    pub fn little_endian(&self, target: Target, vstr: &str) -> String {
        if self.atom_type.repr_size() < 2 || target.uses_reference_target_endianness() {
            vstr.to_string()
        } else {
            format!("___le_{}({})", self.native_type_name, vstr)
        }
    }
}
