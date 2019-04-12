use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

/// The type of a WebAssembly
/// [trap](http://webassembly.github.io/spec/core/intro/overview.html#trap).
#[repr(u32)]
#[derive(Copy, Clone, Debug, FromPrimitive, PartialEq)]
pub enum TrapCode {
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
    Unreachable = 10,
}

impl TrapCode {
    pub fn try_from_u32(v: u32) -> Option<TrapCode> {
        Self::from_u32(v)
    }
}
