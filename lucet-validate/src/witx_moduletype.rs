use crate::{AtomType, FuncSignature};
use witx::{BuiltinType, Datatype, DatatypeIdent, DatatypeVariant, IntRepr, InterfaceFunc};

pub trait HasFuncSignature {
    fn func_signature(&self) -> FuncSignature;
}

impl HasFuncSignature for InterfaceFunc {
    fn func_signature(&self) -> FuncSignature {
        let params = self
            .params
            .iter()
            .flat_map(|p| p.type_.module_types())
            .collect();
        let results = self
            .results
            .iter()
            .flat_map(|p| p.type_.module_types())
            .collect();
        FuncSignature { params, results }
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
            IntRepr::I8 | IntRepr::I16 | IntRepr::I32 => vec![AtomType::I32],
            IntRepr::I64 => vec![AtomType::I64],
        }
    }
}
