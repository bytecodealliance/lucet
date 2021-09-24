use cranelift_codegen::ir;
use cranelift_wasm::{WasmFuncType, WasmType};
use lucet_module::{Signature, ValueType};
use std::fmt::{self, Display};
use thiserror::Error;

#[derive(Debug)]
pub enum ValueError {
    Unrepresentable,
    InvalidVMContext,
}

pub fn value_type(ty: &WasmType) -> ir::types::Type {
    match ty {
        WasmType::I32 => ir::types::I32,
        WasmType::I64 => ir::types::I64,
        WasmType::F32 => ir::types::F32,
        WasmType::F64 => ir::types::F64,
        WasmType::V128 => ir::types::I8X16,
        WasmType::FuncRef | WasmType::ExternRef => ir::types::I64,
        WasmType::ExnRef => unimplemented!(),
    }
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
        .params()
        .iter()
        .map(|paramtype| {
            to_lucet_valuetype(paramtype).map_err(|e| SignatureError::Type(*paramtype, e))
        })
        .collect::<Result<Vec<ValueType>, SignatureError>>()?;

    let ret_ty: Option<ValueType> = match &*func_type.returns() {
        &[] => None,
        &[ref ret_ty] => {
            let value_ty =
                to_lucet_valuetype(ret_ty).map_err(|e| SignatureError::Type(*ret_ty, e))?;

            Some(value_ty)
        }
        _ => {
            return Err(SignatureError::Multivalue);
        }
    };

    Ok(Signature { params, ret_ty })
}
