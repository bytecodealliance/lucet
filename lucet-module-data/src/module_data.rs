use crate::{
    globals::GlobalSpec,
    linear_memory::{HeapSpec, SparseData},
    Error,
};
use serde::{Deserialize, Serialize};

/// The metadata (and some data) for a Lucet module.
///
/// The lifetime parameter exists to support zero-copy deserialization for the `&str` and `&[u8]`
/// fields at the leaves of the structure. For a variant with owned types at the leaves, see
/// [`OwnedModuleData`](struct.OwnedModuleData.html).
///
/// The goal is for this structure to eventually include everything except the code for the guest
/// functions themselves.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleData<'a> {
    heap_spec: HeapSpec,
    #[serde(borrow)]
    sparse_data: SparseData<'a>,
    #[serde(borrow)]
    globals_spec: Vec<GlobalSpec<'a>>,
}

impl<'a> ModuleData<'a> {
    pub fn new(
        heap_spec: HeapSpec,
        sparse_data: SparseData<'a>,
        globals_spec: Vec<GlobalSpec<'a>>,
    ) -> Self {
        Self {
            heap_spec,
            sparse_data,
            globals_spec,
        }
    }

    pub fn heap_spec(&self) -> &HeapSpec {
        &self.heap_spec
    }

    pub fn sparse_data(&self) -> &SparseData<'a> {
        &self.sparse_data
    }

    pub fn globals_spec(&self) -> &[GlobalSpec<'a>] {
        &self.globals_spec
    }

    /// Serialize to (https://github.com/TyOverby/bincode).
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(self).map_err(Error::SerializationError)
    }

    /// Deserialize from [`bincode`](https://github.com/TyOverby/bincode).
    pub fn deserialize(buf: &'a [u8]) -> Result<ModuleData<'a>, Error> {
        bincode::deserialize(buf).map_err(Error::DeserializationError)
    }
}

use crate::{globals::OwnedGlobalSpec, linear_memory::OwnedSparseData};

/// The metadata (and some data) for a Lucet module.
///
/// This is a version of [`ModuleData`](struct.ModuleData.html) with owned types throughout, rather
/// than references to support zero-copy deserialization. This type is useful when directly building
/// up a value to be serialized.
pub struct OwnedModuleData {
    heap_spec: HeapSpec,
    sparse_data: OwnedSparseData,
    globals_spec: Vec<OwnedGlobalSpec>,
}

impl OwnedModuleData {
    pub fn new(
        heap_spec: HeapSpec,
        sparse_data: OwnedSparseData,
        globals_spec: Vec<OwnedGlobalSpec>,
    ) -> Self {
        Self {
            heap_spec,
            sparse_data,
            globals_spec,
        }
    }

    /// Create a [`ModuleData`](struct.ModuleData.html) backed by the values in this
    /// `OwnedModuleData`.
    pub fn to_ref<'a>(&'a self) -> ModuleData<'a> {
        ModuleData::new(
            self.heap_spec.clone(),
            self.sparse_data.to_ref(),
            self.globals_spec.iter().map(|gs| gs.to_ref()).collect(),
        )
    }

    pub fn empty() -> Self {
        Self::new(
            HeapSpec::new(0, 0, 0, None),
            OwnedSparseData::new(vec![]).unwrap(),
            vec![],
        )
    }

    pub fn with_heap_spec(mut self, heap_spec: HeapSpec) -> Self {
        self.heap_spec = heap_spec;
        self
    }
}

impl Default for OwnedModuleData {
    fn default() -> Self {
        OwnedModuleData::empty()
    }
}
