use cranelift_codegen::{ir, isa};
use parity_wasm::elements::{FunctionType, ValueType};

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub sig_ix: u32,
    ftype: FunctionType,
}

impl FunctionSig {
    pub fn new(sig_ix: u32, ftype: &FunctionType) -> Self {
        Self {
            sig_ix: sig_ix,
            ftype: ftype.clone(),
        }
    }
}

pub fn cton_valuetype(t: &ValueType) -> ir::Type {
    match t {
        &ValueType::I32 => ir::types::I32,
        &ValueType::I64 => ir::types::I64,
        &ValueType::F32 => ir::types::F32,
        &ValueType::F64 => ir::types::F64,
        &ValueType::V128 => unimplemented!(),
    }
}

pub trait CtonSignature {
    fn cton_signature(&self) -> ir::Signature;
}

impl CtonSignature for FunctionType {
    fn cton_signature(&self) -> ir::Signature {
        let mut sig = ir::Signature::new(isa::CallConv::SystemV);
        sig.params.insert(
            0,
            ir::AbiParam {
                value_type: ir::types::I64,
                purpose: ir::ArgumentPurpose::VMContext,
                extension: ir::ArgumentExtension::None,
                location: ir::ArgumentLoc::Unassigned,
            },
        );
        sig.params.extend(
            self.params()
                .iter()
                .map(|t| ir::AbiParam::new(cton_valuetype(t))),
        );
        if let Some(t) = self.return_type() {
            sig.returns.push(ir::AbiParam::new(cton_valuetype(&t)));
        }
        sig
    }
}

impl CtonSignature for FunctionSig {
    fn cton_signature(&self) -> ir::Signature {
        self.ftype.cton_signature()
    }
}
