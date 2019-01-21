use crate::bindings::Bindings;
use crate::program::types::{CtonSignature, FunctionSig};
use cranelift_codegen::ir;
use cranelift_module::Linkage;
use failure::Error;
use parity_wasm::elements::{FunctionType, ImportEntry};

pub trait Function {
    fn signature(&self) -> ir::Signature;
    fn signature_index(&self) -> Option<u32>;
    fn linkage(&self) -> Linkage;
    fn symbol(&self) -> &str;
}

impl Function for FunctionImport {
    fn signature(&self) -> ir::Signature {
        self.signature()
    }
    fn signature_index(&self) -> Option<u32> {
        Some(self.signature_index())
    }
    fn linkage(&self) -> Linkage {
        self.linkage()
    }
    fn symbol(&self) -> &str {
        self.symbol()
    }
}

impl Function for FunctionDef {
    fn signature(&self) -> ir::Signature {
        self.signature()
    }
    fn signature_index(&self) -> Option<u32> {
        Some(self.signature_index())
    }
    fn linkage(&self) -> Linkage {
        self.linkage()
    }
    fn symbol(&self) -> &str {
        self.symbol()
    }
}

impl Function for FunctionRuntime {
    fn signature(&self) -> ir::Signature {
        self.signature()
    }
    fn signature_index(&self) -> Option<u32> {
        None
    }
    fn linkage(&self) -> Linkage {
        self.linkage()
    }
    fn symbol(&self) -> &str {
        self.symbol()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub wasmidx: u32,
    sig: FunctionSig,
    exported: bool,
    symbol: String,
}

impl FunctionDef {
    pub fn new(wasmidx: u32, sig: FunctionSig, exported: bool, symbol: String) -> Self {
        Self {
            wasmidx,
            sig,
            exported,
            symbol,
        }
    }

    pub fn signature(&self) -> ir::Signature {
        self.sig.cton_signature()
    }

    pub fn signature_index(&self) -> u32 {
        self.sig.sig_ix
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn linkage(&self) -> Linkage {
        if self.exported {
            Linkage::Export
        } else {
            Linkage::Local
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionImport {
    wasmidx: u32,
    module: String,
    field: String,
    sig: FunctionSig,
    symbol: String,
}

impl FunctionImport {
    pub fn new(
        wasmidx: u32,
        importentry: &ImportEntry,
        sig: FunctionSig,
        bindings: &Bindings,
    ) -> Result<Self, Error> {
        let module = String::from(importentry.module());
        let field = String::from(importentry.field());
        let symbol = bindings.translate(&module, &field)?;
        Ok(Self {
            wasmidx,
            module,
            field,
            sig,
            symbol,
        })
    }

    pub fn module(&self) -> &str {
        self.module.as_str()
    }

    pub fn field(&self) -> &str {
        self.field.as_str()
    }

    pub fn signature(&self) -> ir::Signature {
        self.sig.cton_signature()
    }

    pub fn signature_index(&self) -> u32 {
        self.sig.sig_ix
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn linkage(&self) -> Linkage {
        Linkage::Import
    }
}

#[derive(Debug, Clone)]
pub struct FunctionRuntime {
    ix: u32,
    symbol: String,
    ty: FunctionType,
}

impl FunctionRuntime {
    pub fn new(ix: u32, symbol: &str, ty: FunctionType) -> Self {
        Self {
            ix: ix,
            symbol: String::from(symbol),
            ty: ty,
        }
    }

    pub fn signature(&self) -> ir::Signature {
        self.ty.cton_signature()
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn linkage(&self) -> Linkage {
        Linkage::Import
    }
}
