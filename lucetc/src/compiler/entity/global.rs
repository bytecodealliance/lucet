use cranelift_codegen::ir;

/// Value of WebAssembly global variable
#[derive(Clone, Copy, Debug)]
pub struct GlobalValue {
    pub var: ir::GlobalValue,
    pub ty: ir::Type,
}
