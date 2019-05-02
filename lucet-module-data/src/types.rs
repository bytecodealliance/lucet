use std::convert::TryFrom;
use cranelift_codegen::ir;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug)]
pub enum ValueError {
    Unrepresentable,
    InvalidVMContext
}

impl TryFrom<&ir::AbiParam> for ValueType {
    type Error = ValueError;

    fn try_from(value: &ir::AbiParam) -> Result<Self, Self::Error> {
        match value {
            ir::AbiParam {
                value_type: cranelift_ty,
                purpose: ir::ArgumentPurpose::Normal,
                extension: ir::ArgumentExtension::None,
                location: ir::ArgumentLoc::Unassigned
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
            },
            _ => Err(ValueError::Unrepresentable)
        }
    }
}

/// A signature for a function in a wasm module.
///
/// Note that this does not explicitly name VMContext as a parameter! It is assumed that all wasm
/// functions take VMContext as their first parameter.
// VMContext is not explicitly named specifically so that value types can be tagged with eight bits
// per parameter, letting us perfectly hash functions of up to eight parameters + return values.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Signature {
    pub params: Vec<ValueType>,
    // In the future, wasm may permit this to be a Vec of ValueType
    pub ret_ty: Option<ValueType>,
}

#[derive(Debug)]
pub enum SignatureError {
    BadElement(ir::AbiParam, ValueError),
    BadSignature
}

impl TryFrom<&ir::Signature> for Signature {
    type Error = SignatureError;

    fn try_from(value: &ir::Signature) -> Result<Self, Self::Error> {
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
                    location: ir::ArgumentLoc::Unassigned
                } => {
                    if value.is_int() && value.bits() == 64 {
                        // this is VMContext, so we can move on.
                    } else {
                        return Err(SignatureError::BadElement(param.to_owned(), ValueError::InvalidVMContext));
                    }
                },
                _ => {
                    return Err(SignatureError::BadElement(param.to_owned(), ValueError::InvalidVMContext));
                }
            }
        } else {
            return Err(SignatureError::BadSignature);
        }

        for param in param_iter {
            let value_ty = ValueType::try_from(param)
                .map_err(|e| SignatureError::BadElement(param.clone(), e))?;

            params.push(value_ty);
        }

        let ret_ty: Option<ValueType> = match value.returns.as_slice() {
            &[] => None,
            &[ref ret_ty] => {
                let value_ty = ValueType::try_from(ret_ty)
                    .map_err(|e| SignatureError::BadElement(ret_ty.clone(), e))?;

                Some(value_ty)
            },
            _ => {
                return Err(SignatureError::BadSignature);
            }
        };

        Ok(Signature { params, ret_ty })
    }
}
