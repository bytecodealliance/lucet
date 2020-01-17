use cranelift_codegen::ir;
use lucet_module::Signature;
use lucet_module::ValueType;
use std::fmt::{self, Display};
use thiserror::Error;

#[derive(Debug)]
pub enum ValueError {
    Unrepresentable,
    InvalidVMContext,
}

fn to_lucet_value(value: &ir::AbiParam) -> Result<ValueType, ValueError> {
    match value {
        ir::AbiParam {
            value_type: cranelift_ty,
            purpose: ir::ArgumentPurpose::Normal,
            extension: ir::ArgumentExtension::None,
            location: ir::ArgumentLoc::Unassigned,
        } => {
            let size = cranelift_ty.bits();

            if cranelift_ty.is_int() {
                match size {
                    32 => Ok(ValueType::I32),
                    64 => Ok(ValueType::I64),
                    _ => Err(ValueError::Unrepresentable),
                }
            } else if cranelift_ty.is_float() {
                match size {
                    32 => Ok(ValueType::F32),
                    64 => Ok(ValueType::F64),
                    _ => Err(ValueError::Unrepresentable),
                }
            } else {
                Err(ValueError::Unrepresentable)
            }
        }
        _ => Err(ValueError::Unrepresentable),
    }
}

#[derive(Debug, Error)]
pub enum SignatureError {
    BadElement(ir::AbiParam, ValueError),
    BadSignature,
}

impl Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self, f)
    }
}

pub fn to_lucet_signature(value: &ir::Signature) -> Result<Signature, SignatureError> {
    let mut params: Vec<ValueType> = Vec::new();

    let mut param_iter = value.params.iter();

    // Enforce that the first parameter is VMContext, as Signature assumes.
    // Even functions declared no-arg take VMContext in reality.
    if let Some(param) = param_iter.next() {
        match &param {
            ir::AbiParam {
                value_type: value,
                purpose: ir::ArgumentPurpose::VMContext,
                extension: ir::ArgumentExtension::None,
                location: ir::ArgumentLoc::Unassigned,
            } => {
                if value.is_int() && value.bits() == 64 {
                    // this is VMContext, so we can move on.
                } else {
                    return Err(SignatureError::BadElement(
                        param.to_owned(),
                        ValueError::InvalidVMContext,
                    ));
                }
            }
            _ => {
                return Err(SignatureError::BadElement(
                    param.to_owned(),
                    ValueError::InvalidVMContext,
                ));
            }
        }
    } else {
        return Err(SignatureError::BadSignature);
    }

    for param in param_iter {
        let value_ty =
            to_lucet_value(param).map_err(|e| SignatureError::BadElement(param.clone(), e))?;

        params.push(value_ty);
    }

    let ret_ty: Option<ValueType> = match value.returns.as_slice() {
        &[] => None,
        &[ref ret_ty] => {
            let value_ty = to_lucet_value(ret_ty)
                .map_err(|e| SignatureError::BadElement(ret_ty.clone(), e))?;

            Some(value_ty)
        }
        _ => {
            return Err(SignatureError::BadSignature);
        }
    };

    Ok(Signature { params, ret_ty })
}
