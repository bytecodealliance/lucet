use crate::module_data::ModuleData;
use crate::tables::TableElement;
use crate::functions::FunctionSpec;

pub const LUCET_MODULE_SYM: &str = "lucet_module";

#[derive(Debug)]
pub struct Module<'a> {
    pub module_data: ModuleData<'a>,
    pub tables: &'a [&'a [TableElement]],
    pub function_manifest: &'a [FunctionSpec],
}

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
