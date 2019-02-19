#[macro_use]
extern crate failure;

pub mod globals;
pub mod linear_memory;
pub mod module_data;

pub use crate::globals::{Global, GlobalDef, GlobalImport, GlobalSpec};
pub use crate::linear_memory::{HeapSpec, SparseData};
pub use crate::module_data::ModuleData;

pub mod owned {
    pub use crate::globals::OwnedGlobalSpec;
    pub use crate::linear_memory::OwnedSparseData;
    pub use crate::module_data::OwnedModuleData;
}
