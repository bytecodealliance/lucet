use cranelift_codegen::ir::{types, AbiParam, Signature};
use cranelift_codegen::isa::TargetFrontendConfig;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum RuntimeFunc {
    MemSize,
    MemGrow,
}

pub struct Runtime {
    pub functions: HashMap<RuntimeFunc, (String, Signature)>,
}

impl Runtime {
    pub fn lucet(target: TargetFrontendConfig) -> Self {
        let mut functions = HashMap::new();
        functions.insert(
            RuntimeFunc::MemSize,
            (
                "lucet_vmctx_current_memory".to_owned(),
                Signature {
                    params: vec![],
                    returns: vec![AbiParam::new(types::I32)],
                    call_conv: target.default_call_conv,
                },
            ),
        );
        functions.insert(
            RuntimeFunc::MemGrow,
            (
                "lucet_vmctx_grow_memory".to_owned(),
                Signature {
                    params: vec![
                        AbiParam::new(types::I32), // wasm pages to grow
                    ],
                    returns: vec![AbiParam::new(types::I32)],
                    call_conv: target.default_call_conv,
                },
            ),
        );
        Self { functions }
    }
}
