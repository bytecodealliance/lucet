use crate::error::Error;
use crate::module::{
    AddrDetails, HeapSpec, Module, ModuleInternal, RuntimeSpec, TableElement, TrapManifestRecord,
};
use libc::c_void;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MockModule {
    pub table_elements: Vec<TableElement>,
    sparse_page_data: Vec<Vec<u8>>,
    sparse_page_data_ptrs: Vec<*const c_void>,
    pub runtime_spec: RuntimeSpec,
    pub export_funcs: HashMap<Vec<u8>, *const extern "C" fn()>,
    pub func_table: HashMap<(u32, u32), *const extern "C" fn()>,
    pub start_func: Option<extern "C" fn()>,
    pub trap_manifest: Vec<TrapManifestRecord>,
}

unsafe impl Send for MockModule {}
unsafe impl Sync for MockModule {}

impl MockModule {
    pub fn new() -> Self {
        MockModule {
            table_elements: vec![],
            sparse_page_data: vec![],
            sparse_page_data_ptrs: vec![],
            runtime_spec: RuntimeSpec::default(),
            export_funcs: HashMap::new(),
            func_table: HashMap::new(),
            start_func: None,
            trap_manifest: vec![],
        }
    }

    pub fn arced() -> Arc<dyn Module> {
        Arc::new(MockModule::new())
    }

    pub fn arced_with_heap(heap: &HeapSpec) -> Arc<dyn Module> {
        let mut module = MockModule::new();
        module.runtime_spec.heap = heap.clone();
        Arc::new(module)
    }

    pub fn set_initial_heap(&mut self, heap: &[u8]) {
        self.sparse_page_data.clear();
        self.sparse_page_data_ptrs.clear();
        for page in heap.chunks(4096) {
            let page = page.to_vec();
            self.sparse_page_data_ptrs
                .push(page.as_ptr() as *const c_void);
            self.sparse_page_data.push(page);
        }
    }
}

impl Module for MockModule {}

impl ModuleInternal for MockModule {
    fn table_elements(&self) -> Result<&[TableElement], Error> {
        Ok(&self.table_elements)
    }

    fn sparse_page_data(&self) -> Result<&[*const c_void], Error> {
        Ok(&self.sparse_page_data_ptrs)
    }

    fn runtime_spec(&self) -> &RuntimeSpec {
        &self.runtime_spec
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
