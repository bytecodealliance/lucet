#[macro_use]
extern crate failure;

mod lucet_module_data_capnp {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/lucet_module_data_capnp.rs"));
}

pub mod module_data;
pub mod linear_memory;

pub use crate::module_data::{ModuleData, ModuleDataBox};
pub use crate::linear_memory::{HeapSpec, SparseData};
