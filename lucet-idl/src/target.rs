use crate::types::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Target {
    X86,
    X86_64_64,
    X86_64_32,
    Generic,
}

impl Default for Target {
    fn default() -> Self {
        Target::Generic
    }
}

impl<T: AsRef<str>> From<T> for Target {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "x86" => Target::X86,
            "x86_64_64" | "x86_64" => Target::X86_64_64,
            "x86_64_32" => Target::X86_64_32,
            "generic" => Target::Generic,
            _ => Target::default(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Endianness {
    BigEndian,
    LittleEndian,
    Generic,
}

impl Target {
    pub fn reference_target() -> Target {
        Target::X86_64_64
    }

    pub fn endianness(self) -> Endianness {
        match self {
            Target::Generic => Endianness::Generic,
            _ => Endianness::LittleEndian,
        }
    }

    pub fn is_reference_target(self) -> bool {
        self == Self::reference_target()
    }

    pub fn is_reference_alignment_compatible(self) -> bool {
        self != Target::Generic
    }

    pub fn uses_reference_target_endianness(self) -> bool {
        self.endianness() == Self::reference_target().endianness()
    }

    pub fn uses_reference_target_endianness_for_atom_type(self, atom_type: AtomType) -> bool {
        match atom_type {
            AtomType::U8 | AtomType::I8 => true,
            _ => self.uses_reference_target_endianness(),
        }
    }
}
