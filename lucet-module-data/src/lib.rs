#[macro_use]
extern crate failure;

pub mod module_data;
pub mod linear_memory;
pub mod globals;

pub use crate::module_data::ModuleData;
pub use crate::linear_memory::{HeapSpec, SparseData};
pub use crate::globals::{GlobalSpec, Global, GlobalDef, GlobalImport};
