//! Common types for representing Lucet modules.
//!
//! These types are used both in `lucetc` and `lucet-runtime`, with values serialized in
//! [`bincode`](https://github.com/TyOverby/bincode) format to the compiled Lucet modules.

#![deny(bare_trait_objects)]

pub mod bindings;
pub mod error;
mod functions;
mod globals;
mod linear_memory;
mod module;
mod module_data;
mod runtime;
mod signature;
mod tables;
mod traps;
mod types;
mod version_info;

pub use crate::error::Error;
pub use crate::functions::{
    ExportFunction, FunctionHandle, FunctionIndex, FunctionMetadata, FunctionPointer, FunctionSpec,
    ImportFunction, UniqueSignatureIndex,
};
pub use crate::globals::{Global, GlobalDef, GlobalSpec, GlobalValue};
pub use crate::linear_memory::{HeapSpec, LinearMemorySpec, SparseData};
pub use crate::module::{Module, SerializedModule, LUCET_MODULE_SYM};
pub use crate::module_data::{ModuleData, ModuleFeatures, MODULE_DATA_SYM};
pub use crate::runtime::InstanceRuntimeData;
pub use crate::signature::{ModuleSignature, PublicKey};
pub use crate::tables::TableElement;
pub use crate::traps::{TrapCode, TrapManifest, TrapSite};
pub use crate::types::{Signature, ValueType};
pub use crate::version_info::VersionInfo;

/// Owned variants of the module data types, useful for serialization and testing.
pub mod owned {
    pub use crate::functions::{OwnedExportFunction, OwnedFunctionMetadata, OwnedImportFunction};
    pub use crate::globals::OwnedGlobalSpec;
    pub use crate::linear_memory::{OwnedLinearMemorySpec, OwnedSparseData};
    pub use crate::module_data::OwnedModuleData;
}
