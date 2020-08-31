use cranelift_wasm::{WasmFuncType, WasmType};
use lucet_module::{Signature, ValueType};
use std::fmt::{self, Display};
use thiserror::Error;

#[derive(Debug)]
pub enum ValueError {
    Unrepresentable,
    InvalidVMContext,
}

fn to_lucet_valuetype(ty: &WasmType) -> Result<ValueType, ValueError> {
    match ty {
        WasmType::I32 => Ok(ValueType::I32),
        WasmType::I64 => Ok(ValueType::I64),
        WasmType::F32 => Ok(ValueType::F32),
        WasmType::F64 => Ok(ValueType::F64),
        _ => Err(ValueError::Unrepresentable),
    }
}

#[derive(Debug, Error)]
pub enum SignatureError {
    Type(WasmType, ValueError),
    Multivalue,
}

impl Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self, f)
    }
}

pub fn to_lucet_signature(func_type: &WasmFuncType) -> Result<Signature, SignatureError> {
    let params = func_type
        .params
        .iter()
        .map(|paramtype| {
            to_lucet_valuetype(paramtype).map_err(|e| SignatureError::Type(paramtype.clone(), e))
        })
        .collect::<Result<Vec<ValueType>, SignatureError>>()?;

    let ret_ty: Option<ValueType> = match &*func_type.returns {
        &[] => None,
        &[ref ret_ty] => {
            let value_ty =
                to_lucet_valuetype(ret_ty).map_err(|e| SignatureError::Type(ret_ty.clone(), e))?;

            Some(value_ty)
        }
        _ => {
            return Err(SignatureError::Multivalue);
        }
    };

    Ok(Signature { params, ret_ty })
}
