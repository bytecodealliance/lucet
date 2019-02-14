use crate::{
    globals::GlobalSpec,
    linear_memory::{HeapSpec, SparseData},
};
use failure::Error;
use serde::{Deserialize, Serialize};

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
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(self).map_err(|e| format_err!("serialization error: {}", e))
    }

    pub fn deserialize(buf: &'a [u8]) -> Result<ModuleData<'a>, Error> {
        bincode::deserialize(buf).map_err(|e| format_err!("deserialization error: {}", e))
    }
}

use crate::{globals::OwnedGlobalSpec, linear_memory::OwnedSparseData};

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
    pub fn get_ref(&self) -> ModuleData {
        ModuleData::new(
            self.heap_spec.clone(),
            self.sparse_data.get_ref(),
            self.globals_spec.iter().map(|gs| gs.get_ref()).collect(),
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
