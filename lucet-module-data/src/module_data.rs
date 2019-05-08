use crate::{
    functions::{FunctionMetadata, OwnedFunctionMetadata},
    globals::GlobalSpec,
    linear_memory::{HeapSpec, LinearMemorySpec, SparseData},
    types::Signature,
    Error,
};
use serde::{Deserialize, Serialize};

/// The metadata (and some data) for a Lucet module.
///
/// The lifetime parameter exists to support zero-copy deserialization for the `&str` and `&[u8]`
/// fields at the leaves of the structure. For a variant with owned types at the leaves, see
/// [`OwnedModuleData`](owned/struct.OwnedModuleData.html).
///
/// The goal is for this structure to eventually include everything except the code for the guest
/// functions themselves.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleData<'a> {
    #[serde(borrow)]
    linear_memory: Option<LinearMemorySpec<'a>>,
    #[serde(borrow)]
    globals_spec: Vec<GlobalSpec<'a>>,
    #[serde(borrow)]
    function_info: Vec<FunctionMetadata<'a>>,
    signatures: Vec<Signature>,
}

impl<'a> ModuleData<'a> {
    pub fn new(
        linear_memory: Option<LinearMemorySpec<'a>>,
        globals_spec: Vec<GlobalSpec<'a>>,
        function_info: Vec<FunctionMetadata<'a>>,
        signatures: Vec<Signature>,
    ) -> Self {
        Self {
            linear_memory,
            globals_spec,
            function_info,
            signatures,
        }
    }

    pub fn heap_spec(&self) -> Option<&HeapSpec> {
        if let Some(ref linear_memory) = self.linear_memory {
            Some(&linear_memory.heap)
        } else {
            None
        }
    }

    pub fn sparse_data(&self) -> Option<&SparseData<'a>> {
        if let Some(ref linear_memory) = self.linear_memory {
            Some(&linear_memory.initializer)
        } else {
            None
        }
    }

    pub fn globals_spec(&self) -> &[GlobalSpec<'a>] {
        &self.globals_spec
    }

    // Function index here is a different index space than `get_func_from_idx`, which
    // uses function index as an index into a table of function elements.
    //
    // This is an index of all functions in the module.
    pub fn get_signature(&self, fn_id: u32) -> Option<&Signature> {
        self.function_info.get(fn_id as usize).map(|func| {
            self.signatures().get(func.signature.as_u32() as usize).expect("functions have a signature")
        })
    }

    pub fn get_name(&self, fn_id: u32) -> Option<&str> {
        self.function_info.get(fn_id as usize).and_then(|func| func.name)
    }

    pub fn signatures(&self) -> &[Signature] {
        &self.signatures
    }

    /// Serialize to [`bincode`](https://github.com/TyOverby/bincode).
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(self).map_err(Error::SerializationError)
    }

    /// Deserialize from [`bincode`](https://github.com/TyOverby/bincode).
    pub fn deserialize(buf: &'a [u8]) -> Result<ModuleData<'a>, Error> {
        bincode::deserialize(buf).map_err(Error::DeserializationError)
    }
}

use crate::{
    globals::OwnedGlobalSpec,
    linear_memory::{OwnedLinearMemorySpec, OwnedSparseData},
};

/// The metadata (and some data) for a Lucet module.
///
/// This is a version of [`ModuleData`](../struct.ModuleData.html) with owned types throughout,
/// rather than references to support zero-copy deserialization. This type is useful when directly
/// building up a value to be serialized.
pub struct OwnedModuleData {
    linear_memory: Option<OwnedLinearMemorySpec>,
    globals_spec: Vec<OwnedGlobalSpec>,
    function_info: Vec<OwnedFunctionMetadata>,
    signatures: Vec<Signature>,
}

impl OwnedModuleData {
    pub fn new(
        linear_memory: Option<OwnedLinearMemorySpec>,
        globals_spec: Vec<OwnedGlobalSpec>,
        function_info: Vec<OwnedFunctionMetadata>,
        signatures: Vec<Signature>,
    ) -> Self {
        Self {
            linear_memory,
            globals_spec,
            function_info,
            signatures,
        }
    }

    /// Create a [`ModuleData`](../struct.ModuleData.html) backed by the values in this
    /// `OwnedModuleData`.
    pub fn to_ref<'a>(&'a self) -> ModuleData<'a> {
        ModuleData::new(
            if let Some(ref owned_linear_memory) = self.linear_memory {
                Some(owned_linear_memory.to_ref())
            } else {
                None
            },
            self.globals_spec.iter().map(|gs| gs.to_ref()).collect(),
            self.function_info.iter().map(|info| info.to_ref()).collect(),
            self.signatures.clone(),
        )
    }

    pub fn empty() -> Self {
        Self::new(None, vec![], vec![], vec![])
    }

    pub fn with_heap_spec(mut self, heap_spec: HeapSpec) -> Self {
        if let Some(ref mut linear_memory) = self.linear_memory {
            linear_memory.heap = heap_spec;
        } else {
            self.linear_memory = Some(OwnedLinearMemorySpec {
                heap: heap_spec,
                initializer: OwnedSparseData::new(vec![]).unwrap(),
            });
        }
        self
    }
}

impl Default for OwnedModuleData {
    fn default() -> Self {
        OwnedModuleData::empty()
    }
}
