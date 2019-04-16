//! Common types for representing Lucet module data and metadata.
//!
//! These types are used both in `lucetc` and `lucet-runtime`, with values serialized in
//! [`bincode`](https://github.com/TyOverby/bincode) format to the compiled Lucet modules.

mod error;
mod functions;
mod globals;
mod linear_memory;
mod module_data;
mod traps;

pub use crate::error::Error;
pub use crate::globals::{Global, GlobalDef, GlobalSpec};
pub use crate::linear_memory::{HeapSpec, SparseData, LinearMemorySpec};
pub use crate::module_data::ModuleData;
pub use crate::functions::FunctionSpec;
pub use crate::traps::{TrapManifest, TrapSite, TrapCode};

/// Owned variants of the module data types, useful for serialization and testing.
pub mod owned {
    pub use crate::globals::OwnedGlobalSpec;
    pub use crate::linear_memory::{OwnedSparseData, OwnedLinearMemorySpec};
    pub use crate::module_data::OwnedModuleData;
}
