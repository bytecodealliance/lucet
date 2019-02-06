use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// The details associated with a WebAssembly
/// [trap](http://webassembly.github.io/spec/core/intro/overview.html#trap).
#[derive(Copy, Clone, Debug)]
pub struct TrapCode {
    pub ty: TrapCodeType,
    /// Mainly for internal testing, this field will likely be deprecated soon.
    pub tag: u16,
}

impl std::fmt::Display for TrapCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.ty == TrapCodeType::User {
            write!(f, "{:?}({})", self.ty, self.tag)
        } else {
            write!(f, "{:?}", self.ty)
        }
    }
}

impl TrapCode {
    pub fn try_from_u32(trapcode_bin: u32) -> Option<TrapCode> {
        let trapcode_type = (trapcode_bin & 0x0000FFFF) as u16;
        TrapCodeType::from_u16(trapcode_type).map(|ty| {
            let tag = (trapcode_bin >> 16) as u16;
            TrapCode { ty, tag }
        })
    }
}

/// The type of a WebAssembly
/// [trap](http://webassembly.github.io/spec/core/intro/overview.html#trap).
#[repr(u16)]
#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq)]
pub enum TrapCodeType {
    StackOverflow = 0,
    HeapOutOfBounds = 1,
    OutOfBounds = 2,
    IndirectCallToNull = 3,
    BadSignature = 4,
    IntegerOverflow = 5,
    IntegerDivByZero = 6,
    BadConversionToInteger = 7,
    Interrupt = 8,
    TableOutOfBounds = 9,
    User = 0xFFFF,
    Unknown = 0xFFFE,
}
