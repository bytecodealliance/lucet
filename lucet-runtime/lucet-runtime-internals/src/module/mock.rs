use crate::module::{
    AddrDetails, HeapSpec, Module, ModuleInternal, RuntimeSpec, TableElement, TrapManifestRecord,
};
use failure::{format_err, Error};
use libc::c_void;
use std::collections::HashMap;

pub struct MockModule {
    pub table_elements: Vec<TableElement>,
    pub sparse_page_data: Vec<*const c_void>,
    pub runtime_spec: RuntimeSpec,
    pub export_funcs: HashMap<Vec<u8>, *const extern "C" fn()>,
    pub start_func: Option<extern "C" fn()>,
    pub trap_manifest: Vec<TrapManifestRecord>,
}

impl MockModule {
    pub fn new() -> Self {
        MockModule {
            table_elements: vec![],
            sparse_page_data: vec![],
            runtime_spec: RuntimeSpec::default(),
            export_funcs: HashMap::new(),
            start_func: None,
            trap_manifest: vec![],
        }
    }

    pub fn boxed() -> Box<dyn Module> {
        Box::new(MockModule::new())
    }

    pub fn boxed_with_heap(heap: &HeapSpec) -> Box<dyn Module> {
        let mut module = MockModule::new();
        module.runtime_spec.heap = heap.clone();
        Box::new(module)
    }
}

impl Module for MockModule {}

impl ModuleInternal for MockModule {
    fn table_elements(&self) -> Result<&[TableElement], Error> {
        Ok(&self.table_elements)
    }

    fn sparse_page_data(&self) -> Result<&[*const c_void], Error> {
        Ok(&self.sparse_page_data)
    }

    fn runtime_spec(&self) -> &RuntimeSpec {
        &self.runtime_spec
    }

    fn get_export_func(&self, sym: &[u8]) -> Result<*const extern "C" fn(), Error> {
        let func = self.export_funcs.get(sym).ok_or(format_err!(
            "export func not found: {}",
            String::from_utf8_lossy(sym)
        ))?;
        // eprintln!("{} at {:p}", String::from_utf8_lossy(sym), *func);
        Ok(*func)
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
