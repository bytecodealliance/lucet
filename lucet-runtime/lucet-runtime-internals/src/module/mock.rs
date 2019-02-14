use crate::error::Error;
use crate::module::{
    AddrDetails, Module, ModuleData, ModuleInternal, TableElement, TrapManifestRecord,
};
use libc::c_void;
pub use lucet_module_data::module_data::OwnedModuleData;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MockModule {
    // Need to have OwnedModuleData, the memory backing the ModuleData refs inside this structure.
    // It is only used through the module_data structure.
    #[allow(dead_code)]
    owned_module_data: OwnedModuleData,
    // module_data has the same lifetime as this struct. We
    // mark it as static because MockModule can't have a lifetime
    // parameter (because it needs to be castable into a `&dyn Module`
    // trait object)
    module_data: ModuleData<'static>,

    pub table_elements: Vec<TableElement>,
    pub export_funcs: HashMap<Vec<u8>, *const extern "C" fn()>,
    pub func_table: HashMap<(u32, u32), *const extern "C" fn()>,
    pub start_func: Option<extern "C" fn()>,
    pub trap_manifest: Vec<TrapManifestRecord>,
}

unsafe impl Send for MockModule {}
unsafe impl Sync for MockModule {}

impl MockModule {
    pub fn new(owned_module_data: OwnedModuleData) -> Self {
        let module_data = unsafe { std::mem::transmute(owned_module_data.get_ref()) };
        MockModule {
            module_data,
            owned_module_data,
            table_elements: vec![],
            export_funcs: HashMap::new(),
            func_table: HashMap::new(),
            start_func: None,
            trap_manifest: vec![],
        }
    }

    pub fn arced(self) -> Arc<dyn Module> {
        Arc::new(self)
    }
}

impl Module for MockModule {}

impl ModuleInternal for MockModule {
    fn table_elements(&self) -> Result<&[TableElement], Error> {
        Ok(&self.table_elements)
    }

    fn module_data(&self) -> &ModuleData {
        &self.module_data
    }

    fn get_export_func(&self, sym: &[u8]) -> Result<*const extern "C" fn(), Error> {
        self.export_funcs
            .get(sym)
            .cloned()
            .ok_or(Error::SymbolNotFound(
                String::from_utf8_lossy(sym).into_owned(),
            ))
    }

    fn get_func_from_idx(
        &self,
        table_id: u32,
        func_id: u32,
    ) -> Result<*const extern "C" fn(), Error> {
        self.func_table
            .get(&(table_id, func_id))
            .cloned()
            .ok_or(Error::FuncNotFound(table_id, func_id))
    }

    fn get_start_func(&self) -> Result<Option<*const extern "C" fn()>, Error> {
        Ok(self.start_func.map(|start| start as *const extern "C" fn()))
    }

    fn trap_manifest(&self) -> &[TrapManifestRecord] {
        &self.trap_manifest
    }

    fn addr_details(&self, _addr: *const c_void) -> Result<Option<AddrDetails>, Error> {
        // TODO: possible to reflect on size of Rust functions?
        Ok(None)
    }
}

pub trait ToMockModule {
    fn into_mock(self) -> MockModule;
}

impl ToMockModule for OwnedModuleData {
    fn into_mock(self) -> MockModule {
        MockModule::new(self)
    }
}
