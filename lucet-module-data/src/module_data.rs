use crate::{
    globals::GlobalSpec,
    linear_memory::{HeapSpec, SparseData},
};
use serde::{Serialize, Deserialize};
use failure::Error;

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
