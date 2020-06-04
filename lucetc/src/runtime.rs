use cranelift_codegen::ir::{types, AbiParam, Signature};
use cranelift_codegen::isa::TargetFrontendConfig;
use std::collections::HashMap;
use wasmparser::FuncType;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum RuntimeFunc {
    MemSize,
    MemGrow,
}

pub struct RuntimeFuncType {
    pub name: String,
    pub signature: Signature,
    pub wasm_func_type: FuncType,
}

pub struct Runtime {
    pub functions: HashMap<RuntimeFunc, RuntimeFuncType>,
}

impl Runtime {
    pub fn lucet(target: TargetFrontendConfig) -> Self {
        let mut functions = HashMap::new();
        functions.insert(
            RuntimeFunc::MemSize,
            RuntimeFuncType {
                name: "lucet_vmctx_current_memory".to_owned(),
                signature: Signature {
                    params: vec![],
                    returns: vec![AbiParam::new(types::I32)],
                    call_conv: target.default_call_conv,
                },
                wasm_func_type: FuncType {
                    params: vec![].into_boxed_slice(),
                    returns: vec![wasmparser::Type::I32].into_boxed_slice(),
                },
            },
        );
        functions.insert(
            RuntimeFunc::MemGrow,
            RuntimeFuncType {
                name: "lucet_vmctx_grow_memory".to_owned(),
                signature: Signature {
                    params: vec![
                        AbiParam::new(types::I32), // wasm pages to grow
                    ],
                    returns: vec![AbiParam::new(types::I32)],
                    call_conv: target.default_call_conv,
                },
                wasm_func_type: FuncType {
                    params: vec![wasmparser::Type::I32].into_boxed_slice(),
                    returns: vec![wasmparser::Type::I32].into_boxed_slice(),
                },
            },
        );
        Self { functions }
    }
}
