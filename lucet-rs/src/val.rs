use lucet_sys::*;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Val {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Usize(usize),
    Isize(isize),
    GuestPtr(u32),
    F32(f32),
    F64(f64),
}

impl From<Val> for lucet_val {
    fn from(v: Val) -> Self {
        match v {
            Val::GuestPtr(a) => lucet_val {
                type_: lucet_val_type_lucet_val_guest_ptr,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::U8(a) => lucet_val {
                type_: lucet_val_type_lucet_val_u8,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::U16(a) => lucet_val {
                type_: lucet_val_type_lucet_val_u16,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::U32(a) => lucet_val {
                type_: lucet_val_type_lucet_val_u32,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::U64(a) => lucet_val {
                type_: lucet_val_type_lucet_val_u64,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::I8(a) => lucet_val {
                type_: lucet_val_type_lucet_val_i8,
                inner_val: lucet_val_inner_val { as_i64: a as _ },
            },
            Val::I16(a) => lucet_val {
                type_: lucet_val_type_lucet_val_i16,
                inner_val: lucet_val_inner_val { as_i64: a as _ },
            },
            Val::I32(a) => lucet_val {
                type_: lucet_val_type_lucet_val_i32,
                inner_val: lucet_val_inner_val { as_i64: a as _ },
            },
            Val::I64(a) => lucet_val {
                type_: lucet_val_type_lucet_val_i64,
                inner_val: lucet_val_inner_val { as_i64: a as _ },
            },
            Val::Usize(a) => lucet_val {
                type_: lucet_val_type_lucet_val_usize,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::Isize(a) => lucet_val {
                type_: lucet_val_type_lucet_val_isize,
                inner_val: lucet_val_inner_val { as_i64: a as _ },
            },
            Val::Bool(a) => lucet_val {
                type_: lucet_val_type_lucet_val_bool,
                inner_val: lucet_val_inner_val { as_u64: a as _ },
            },
            Val::F32(a) => lucet_val {
                type_: lucet_val_type_lucet_val_f32,
                inner_val: lucet_val_inner_val { as_f32: a as _ },
            },
            Val::F64(a) => lucet_val {
                type_: lucet_val_type_lucet_val_f64,
                inner_val: lucet_val_inner_val { as_f64: a as _ },
            },
        }
    }
}

#[allow(non_upper_case_globals)]
impl From<lucet_val> for Val {
    fn from(v: lucet_val) -> Self {
        match v.type_ {
            lucet_val_type_lucet_val_guest_ptr => Val::GuestPtr(unsafe { v.inner_val.as_u64 } as _),
            lucet_val_type_lucet_val_u8 => Val::U8(unsafe { v.inner_val.as_u64 } as _),
            lucet_val_type_lucet_val_u16 => Val::U16(unsafe { v.inner_val.as_u64 } as _),
            lucet_val_type_lucet_val_u32 => Val::U32(unsafe { v.inner_val.as_u64 } as _),
            lucet_val_type_lucet_val_u64 => Val::U64(unsafe { v.inner_val.as_u64 } as _),
            lucet_val_type_lucet_val_i8 => Val::I16(unsafe { v.inner_val.as_i64 } as _),
            lucet_val_type_lucet_val_i16 => Val::I32(unsafe { v.inner_val.as_i64 } as _),
            lucet_val_type_lucet_val_i32 => Val::I32(unsafe { v.inner_val.as_i64 } as _),
            lucet_val_type_lucet_val_i64 => Val::I64(unsafe { v.inner_val.as_i64 } as _),
            lucet_val_type_lucet_val_usize => Val::Usize(unsafe { v.inner_val.as_u64 } as _),
            lucet_val_type_lucet_val_isize => Val::Isize(unsafe { v.inner_val.as_i64 } as _),
            lucet_val_type_lucet_val_bool => Val::Bool(unsafe { v.inner_val.as_u64 } != 0),
            lucet_val_type_lucet_val_f32 => Val::F32(unsafe { v.inner_val.as_f32 } as _),
            lucet_val_type_lucet_val_f64 => Val::F64(unsafe { v.inner_val.as_f64 } as _),
            _ => panic!("Unsupported type"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct UntypedRetval(lucet_untyped_retval);

impl From<lucet_untyped_retval> for UntypedRetval {
    fn from(v: lucet_untyped_retval) -> Self {
        UntypedRetval(v)
    }
}

impl fmt::Debug for UntypedRetval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UntypedRetval {{ inner_val.as_u64(): {:?} }}",
            self.as_u64()
        )
    }
}

impl UntypedRetval {
    pub fn as_guest_ptr(&self) -> u32 {
        unsafe { lucet_retval_gp(&self.0).as_u64 as _ }
    }
    pub fn as_u8(&self) -> u8 {
        unsafe { lucet_retval_gp(&self.0).as_u64 as _ }
    }
    pub fn as_u16(&self) -> u16 {
        unsafe { lucet_retval_gp(&self.0).as_u64 as _ }
    }
    pub fn as_u32(&self) -> u32 {
        unsafe { lucet_retval_gp(&self.0).as_u64 as _ }
    }
    pub fn as_u64(&self) -> u64 {
        unsafe { lucet_retval_gp(&self.0).as_u64 as _ }
    }
    pub fn as_i8(&self) -> i8 {
        unsafe { lucet_retval_gp(&self.0).as_i64 as _ }
    }
    pub fn as_i16(&self) -> i16 {
        unsafe { lucet_retval_gp(&self.0).as_i64 as _ }
    }
    pub fn as_i32(&self) -> i32 {
        unsafe { lucet_retval_gp(&self.0).as_i64 as _ }
    }
    pub fn as_i64(&self) -> i64 {
        unsafe { lucet_retval_gp(&self.0).as_i64 as _ }
    }
    pub fn as_usize(&self) -> usize {
        unsafe { lucet_retval_gp(&self.0).as_u64 as _ }
    }
    pub fn as_isize(&self) -> isize {
        unsafe { lucet_retval_gp(&self.0).as_i64 as _ }
    }
    pub fn as_bool(&self) -> bool {
        unsafe { lucet_retval_gp(&self.0).as_u64 != 0 }
    }
    pub fn as_f32(&self) -> f32 {
        unsafe { lucet_retval_f32(&self.0) as _ }
    }
    pub fn as_f64(&self) -> f64 {
        unsafe { lucet_retval_f64(&self.0) as _ }
    }
}
