use cranelift_codegen::ir;

#[cfg(target_pointer_width = "64")]
pub const NATIVE_POINTER: ir::Type = ir::types::I64;
#[cfg(target_pointer_width = "32")]
pub const NATIVE_POINTER: ir::Type = ir::types::I32;
#[cfg(target_pointer_width = "64")]
pub const NATIVE_POINTER_SIZE: usize = 8;
#[cfg(target_pointer_width = "32")]
pub const NATIVE_POINTER_SIZE: usize = 4;
