use crate::{AtomType, FuncSignature};
use failure::Fail;
use witx::{BuiltinType, Datatype, DatatypeIdent, DatatypeVariant, IntRepr, InterfaceFunc};

#[derive(Debug, Fail)]
pub enum SignatureError {
    #[fail(display = "invalid result type: {}", _0)]
    InvalidResultType(String),
}

pub trait HasFuncSignature {
    fn func_signature(&self) -> Result<FuncSignature, SignatureError>;
}

impl HasFuncSignature for InterfaceFunc {
    fn func_signature(&self) -> Result<FuncSignature, SignatureError> {
        let mut params = self
            .params
            .iter()
            .flat_map(|p| p.type_.param_by_value_type())
            .collect::<Vec<AtomType>>();

        let results = if let Some(first_result) = self.results.iter().next() {
            vec![first_result.type_.result_type().ok_or_else(|| {
                SignatureError::InvalidResultType(format!("no result type for {:?}", first_result))
            })?]
        } else {
            vec![]
        };

        let subsequent_results = self
            .results
            .iter()
            .skip(1)
            .flat_map(|p| p.type_.param_by_reference_type());
        params.extend(subsequent_results);
        Ok(FuncSignature { params, results })
    }
}

pub trait ModuleTypeParams {
    fn param_by_value_type(&self) -> Vec<AtomType>;
    fn param_by_reference_type(&self) -> Vec<AtomType>;
    fn result_type(&self) -> Option<AtomType> {
        let mut param_types = self.param_by_value_type();
        match param_types.len() {
            1 => Some(param_types.pop().unwrap()),
            _ => None,
        }
    }
}

impl ModuleTypeParams for DatatypeIdent {
    fn param_by_value_type(&self) -> Vec<AtomType> {
        use DatatypeIdent::*;
        match self {
            Builtin(builtin_type) => match builtin_type {
                BuiltinType::String | BuiltinType::Data => vec![AtomType::I32, AtomType::I32],
                BuiltinType::U8
                | BuiltinType::U16
                | BuiltinType::U32
                | BuiltinType::S8
                | BuiltinType::S16
                | BuiltinType::S32 => vec![AtomType::I32],
                BuiltinType::U64 | BuiltinType::S64 => vec![AtomType::I64],
                BuiltinType::F32 => vec![AtomType::F32],
                BuiltinType::F64 => vec![AtomType::F64],
            },
            Array(_) => vec![AtomType::I32, AtomType::I32],
            Pointer(_) | ConstPointer(_) => vec![AtomType::I32],
            Ident(datatype) => datatype.param_by_value_type(),
        }
    }
    fn param_by_reference_type(&self) -> Vec<AtomType> {
        use DatatypeIdent::*;
        match self {
            Builtin(builtin) => match builtin {
                BuiltinType::String | BuiltinType::Data => self.param_by_value_type(),
                _ => vec![AtomType::I32],
            },
            Array(_) | Pointer(_) | ConstPointer(_) => self.param_by_value_type(),
            Ident(datatype) => datatype.param_by_reference_type(),
        }
    }
}
impl ModuleTypeParams for Datatype {
    fn param_by_value_type(&self) -> Vec<AtomType> {
        match &self.variant {
            DatatypeVariant::Alias(a) => a.to.param_by_value_type(),
            DatatypeVariant::Enum(e) => e.repr.param_by_value_type(),
            DatatypeVariant::Flags(f) => f.repr.param_by_value_type(),
            DatatypeVariant::Struct(_) | DatatypeVariant::Union(_) => vec![AtomType::I32],
        }
    }
    fn param_by_reference_type(&self) -> Vec<AtomType> {
        match &self.variant {
            DatatypeVariant::Alias(a) => a.to.param_by_reference_type(),
            _ => vec![AtomType::I32],
        }
    }
}

impl ModuleTypeParams for IntRepr {
    fn param_by_value_type(&self) -> Vec<AtomType> {
        match self {
            IntRepr::U8 | IntRepr::U16 | IntRepr::U32 => vec![AtomType::I32],
            IntRepr::U64 => vec![AtomType::I64],
        }
    }
    fn param_by_reference_type(&self) -> Vec<AtomType> {
        vec![AtomType::I32]
    }
}
