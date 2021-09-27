use cranelift_codegen::ir::{types, AbiParam, Signature};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{WasmFuncType, WasmType};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum RuntimeFunc {
    MemSize,
    MemGrow,
    YieldAtBoundExpiration,
}

pub struct RuntimeFuncType {
    pub name: String,
    pub signature: Signature,
    pub wasm_func_type: WasmFuncType,
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
                wasm_func_type: WasmFuncType::new(
                    vec![].into_boxed_slice(),
                    vec![WasmType::I32].into_boxed_slice(),
                ),
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
                wasm_func_type: WasmFuncType::new(
                    vec![WasmType::I32].into_boxed_slice(),
                    vec![WasmType::I32].into_boxed_slice(),
                ),
            },
        );
        functions.insert(
            RuntimeFunc::YieldAtBoundExpiration,
            RuntimeFuncType {
                name: "lucet_vmctx_yield_at_bound_expiration".to_owned(),
                signature: Signature {
                    params: vec![],
                    returns: vec![],
                    call_conv: target.default_call_conv,
                },
                wasm_func_type: WasmFuncType::new(
                    vec![].into_boxed_slice(),
                    vec![].into_boxed_slice(),
                ),
            },
        );
        Self { functions }
    }
}
