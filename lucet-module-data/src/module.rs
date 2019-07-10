use crate::functions::FunctionSpec;
use crate::module_data::ModuleData;
use crate::tables::TableElement;

pub const LUCET_MODULE_SYM: &str = "lucet_module";

/// Module is the exposed structure that contains all the data backing a Lucet-compiled object.
#[derive(Debug)]
pub struct Module<'a> {
    pub module_data: ModuleData<'a>,
    pub tables: &'a [&'a [TableElement]],
    pub function_manifest: &'a [FunctionSpec],
}

/// NativeData is a serialization-friendly form of Module, in that `ModuleData` is serialized, with
/// the `module_data_*` fields here referring to that serialized data, while `tables_*` and
/// `function_manifest_*` refer to the actual tables and function manifest data written in the
/// binary.
#[repr(C)]
#[derive(Debug)]
pub struct NativeData {
    pub module_data_ptr: u64,
    pub module_data_len: u64,
    pub tables_ptr: u64,
    pub tables_len: u64,
    pub function_manifest_ptr: u64,
    pub function_manifest_len: u64,
}
