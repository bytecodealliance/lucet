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
            .flat_map(|p| p.type_.module_types())
            .collect::<Vec<AtomType>>();

        let first_result = self.results.iter().next().map(|p| {
            let mut ts = p.type_.module_types();
            if ts.len() > 1 {
                Err(SignatureError::InvalidResultType(format!(
                    "in {}: result {}: {:?} represented as module types {:?}",
                    self.name.as_str(),
                    p.name.as_str(),
                    p.type_,
                    ts
                )))
            } else {
                Ok(ts.pop().unwrap())
            }
        });
        let results = if let Some(r) = first_result {
            vec![r?.clone()]
        } else {
            vec![]
        };

        let subsequent_results = self
            .results
            .iter()
            .skip(1)
            .flat_map(|p| p.type_.module_types());
        params.extend(subsequent_results);
        Ok(FuncSignature { params, results })
    }
}

pub trait ModuleTypeParams {
    fn module_types(&self) -> Vec<AtomType>;
}

impl ModuleTypeParams for DatatypeIdent {
    fn module_types(&self) -> Vec<AtomType> {
        match self {
            DatatypeIdent::Builtin(builtin_type) => match builtin_type {
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
            DatatypeIdent::Array(_) => vec![AtomType::I32, AtomType::I32],
            DatatypeIdent::Pointer(_) | DatatypeIdent::ConstPointer(_) => vec![AtomType::I32],
            DatatypeIdent::Ident(datatype) => datatype.module_types(),
        }
    }
}
impl ModuleTypeParams for Datatype {
    fn module_types(&self) -> Vec<AtomType> {
        match &self.variant {
            DatatypeVariant::Alias(a) => a.to.module_types(),
            DatatypeVariant::Enum(e) => e.repr.module_types(),
            DatatypeVariant::Flags(f) => f.repr.module_types(),
            DatatypeVariant::Struct(_) | DatatypeVariant::Union(_) => vec![AtomType::I32],
        }
    }
}

impl ModuleTypeParams for IntRepr {
    fn module_types(&self) -> Vec<AtomType> {
        match self {
            IntRepr::U8 | IntRepr::U16 | IntRepr::U32 => vec![AtomType::I32],
            IntRepr::U64 => vec![AtomType::I64],
        }
    }
}
