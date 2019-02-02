//! Typed values for passing into and returning from sandboxed
//! programs.

use libc::{
    c_char, c_int, c_long, c_longlong, c_short, c_uchar, c_uint, c_ulong, c_ulonglong, c_ushort,
    c_void,
};
use std::arch::x86_64::{
    __m128, _mm_castpd_ps, _mm_castps_pd, _mm_load_pd1, _mm_load_ps1, _mm_setzero_ps,
    _mm_storeu_pd, _mm_storeu_ps,
};

/// Typed values used for passing arguments into new contexts, and for
/// reading return values from completed contexts.
///
/// TODO: Why do we have a set of both Rust and C integers, but only
/// Rust floats? When should one or the other be used?
#[derive(Clone, Copy, Debug)]
pub enum Val {
    CPtr(*const c_void),
    /// A WebAssembly linear memory address
    GuestPtr(u32),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    USize(usize),
    ISize(isize),
    CUChar(c_uchar),
    CUShort(c_ushort),
    CUInt(c_uint),
    CULong(c_ulong),
    CULongLong(c_ulonglong),
    CChar(c_char),
    CShort(c_short),
    CInt(c_int),
    CLong(c_long),
    CLongLong(c_longlong),
    Bool(bool),
    F32(f32),
    F64(f64),
}

macro_rules! impl_from_scalars {
    ( { $( $ctor:ident : $ty:ty ),* } ) => {
        $(
            impl From<$ty> for Val {
                fn from(x: $ty) -> Val {
                    Val::$ctor(x)
                }
            }
        )*
    };
}

// Since there is overlap in these enum variants, we can't have instances for all of them, such as
// GuestPtr and the C type aliases
impl_from_scalars!({
    CPtr: *const c_void,
    CPtr: *mut c_void,
    U8: u8,
    U16: u16,
    U32: u32,
    U64: u64,
    I8: i8,
    I16: i16,
    I32: i32,
    I64: i64,
    USize: usize,
    ISize: isize,
    Bool: bool,
    F32: f32,
    F64: f64
});

/// Register representation of `Val`.
///
/// When mapping `Val`s to x86_64 registers, we map floating point
/// values into the SSE registers _xmmN_, and all other values into
/// general-purpose (integer) registers.
pub enum RegVal {
    GpReg(u64),
    FpReg(__m128),
}

impl Val {
    /// Convert a `Val` to its representation when stored in an
    /// argument register.
    pub fn to_reg(&self) -> RegVal {
        use self::RegVal::*;
        use self::Val::*;
        match *self {
            CPtr(v) => GpReg(v as u64),
            GuestPtr(v) => GpReg(v as u64),
            U8(v) => GpReg(v as u64),
            U16(v) => GpReg(v as u64),
            U32(v) => GpReg(v as u64),
            U64(v) => GpReg(v as u64),
            I8(v) => GpReg(v as u64),
            I16(v) => GpReg(v as u64),
            I32(v) => GpReg(v as u64),
            I64(v) => GpReg(v as u64),
            USize(v) => GpReg(v as u64),
            ISize(v) => GpReg(v as u64),
            CUChar(v) => GpReg(v as u64),
            CUShort(v) => GpReg(v as u64),
            CUInt(v) => GpReg(v as u64),
            CULong(v) => GpReg(v as u64),
            CULongLong(v) => GpReg(v as u64),
            CChar(v) => GpReg(v as u64),
            CShort(v) => GpReg(v as u64),
            CInt(v) => GpReg(v as u64),
            CLong(v) => GpReg(v as u64),
            CLongLong(v) => GpReg(v as u64),
            Bool(false) => GpReg(0u64),
            Bool(true) => GpReg(1u64),
            Val::F32(v) => FpReg(unsafe { _mm_load_ps1(&v as *const f32) }),
            Val::F64(v) => FpReg(unsafe { _mm_castpd_ps(_mm_load_pd1(&v as *const f64)) }),
        }
    }

    /// Convert a `Val` to its representation when spilled onto the
    /// stack.
    pub fn to_stack(&self) -> u64 {
        use self::Val::*;
        match *self {
            CPtr(v) => v as u64,
            GuestPtr(v) => v as u64,
            U8(v) => v as u64,
            U16(v) => v as u64,
            U32(v) => v as u64,
            U64(v) => v as u64,
            I8(v) => v as u64,
            I16(v) => v as u64,
            I32(v) => v as u64,
            I64(v) => v as u64,
            USize(v) => v as u64,
            ISize(v) => v as u64,
            CUChar(v) => v as u64,
            CUShort(v) => v as u64,
            CUInt(v) => v as u64,
            CULong(v) => v as u64,
            CULongLong(v) => v as u64,
            CChar(v) => v as u64,
            CShort(v) => v as u64,
            CInt(v) => v as u64,
            CLong(v) => v as u64,
            CLongLong(v) => v as u64,
            Bool(false) => 0u64,
            Bool(true) => 1u64,
            F32(v) => v.to_bits() as u64,
            F64(v) => v.to_bits(),
        }
    }
}

/// An untyped value returned by guest function calls.
#[derive(Clone, Copy, Debug)]
pub struct UntypedRetVal {
    fp: __m128,
    gp: u64,
}

impl std::fmt::Display for UntypedRetVal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<untyped return value>")
    }
}

impl UntypedRetVal {
    pub fn new(gp: u64, fp: __m128) -> UntypedRetVal {
        UntypedRetVal { gp, fp }
    }
}

macro_rules! impl_from_fp {
    ( $ty:ty, $f:ident ) => {
        impl From<UntypedRetVal> for $ty {
            fn from(retval: UntypedRetVal) -> $ty {
                $f(retval.fp)
            }
        }

        impl From<&UntypedRetVal> for $ty {
            fn from(retval: &UntypedRetVal) -> $ty {
                $f(retval.fp)
            }
        }
    };
}

impl_from_fp!(f32, __m128_as_f32);
impl_from_fp!(f64, __m128_as_f64);

macro_rules! impl_from_gp {
    ( $ty:ty ) => {
        impl From<UntypedRetVal> for $ty {
            fn from(retval: UntypedRetVal) -> $ty {
                retval.gp as $ty
            }
        }

        impl From<&UntypedRetVal> for $ty {
            fn from(retval: &UntypedRetVal) -> $ty {
                retval.gp as $ty
            }
        }
    };
}

impl_from_gp!(u8);
impl_from_gp!(u16);
impl_from_gp!(u32);
impl_from_gp!(u64);

impl_from_gp!(i8);
impl_from_gp!(i16);
impl_from_gp!(i32);
impl_from_gp!(i64);

impl From<UntypedRetVal> for bool {
    fn from(retval: UntypedRetVal) -> bool {
        retval.gp != 0
    }
}

impl From<&UntypedRetVal> for bool {
    fn from(retval: &UntypedRetVal) -> bool {
        retval.gp != 0
    }
}

impl Default for UntypedRetVal {
    fn default() -> UntypedRetVal {
        let fp = unsafe { _mm_setzero_ps() };
        UntypedRetVal { fp, gp: 0 }
    }
}

// Helpers that we might want to put in a utils module someday

/// Interpret the contents of a `__m128` register as an `f32`.
pub fn __m128_as_f32(v: __m128) -> f32 {
    let mut out: [f32; 4] = [0.0; 4];
    unsafe {
        _mm_storeu_ps(&mut out[0] as *mut f32, v);
    }
    out[0]
}

/// Interpret the contents of a `__m128` register as an `f64`.
pub fn __m128_as_f64(v: __m128) -> f64 {
    let mut out: [f64; 2] = [0.0; 2];
    unsafe {
        let vd = _mm_castps_pd(v);
        _mm_storeu_pd(&mut out[0] as *mut f64, vd);
    }
    out[0]
}
