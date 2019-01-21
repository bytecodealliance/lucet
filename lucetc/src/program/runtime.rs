use super::function::FunctionRuntime;
use crate::error::LucetcError;
use failure::format_err;
use parity_wasm::elements::{FunctionType, ValueType};

#[derive(Debug, Clone)]
pub struct Runtime {
    funcs: Vec<FunctionRuntime>,
}

impl Runtime {
    pub fn liblucet_runtime_c() -> Self {
        let current_memory_type = FunctionType::new(Vec::new(), Some(ValueType::I32));
        let grow_memory_type = FunctionType::new(vec![ValueType::I32], Some(ValueType::I32));
        Self {
            funcs: vec![
                FunctionRuntime::new(0, "lucet_vmctx_current_memory", current_memory_type),
                FunctionRuntime::new(1, "lucet_vmctx_grow_memory", grow_memory_type),
            ],
        }
    }

    pub fn functions(&self) -> &[FunctionRuntime] {
        self.funcs.as_ref()
    }

    pub fn get_index(&self, ix: u32) -> Result<FunctionRuntime, LucetcError> {
        let rt = self
            .funcs
            .get(ix as usize)
            .map(|f| f.clone())
            .ok_or(format_err!("runtime function {} out of bounds", ix))?;
        Ok(rt)
    }

    pub fn get_symbol(&self, symbol: &str) -> Result<&FunctionRuntime, LucetcError> {
        let symbol = String::from(symbol);
        for f in self.funcs.iter() {
            if f.symbol() == symbol {
                return Ok(f);
            }
        }
        Err(format_err!("runtime function \"{}\" not found", symbol))?
    }
}
