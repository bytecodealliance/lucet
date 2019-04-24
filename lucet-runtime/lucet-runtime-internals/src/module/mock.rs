use crate::error::Error;
use crate::module::{
    AddrDetails, GlobalSpec, HeapSpec, Module, ModuleInternal, TableElement, TrapManifestRecord,
};
use libc::c_void;
use lucet_module_data::owned::{
    OwnedGlobalSpec, OwnedLinearMemorySpec, OwnedModuleData, OwnedSparseData,
};
use lucet_module_data::ModuleData;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Default)]
pub struct MockModuleBuilder {
    heap_spec: HeapSpec,
    sparse_page_data: Vec<Option<Vec<u8>>>,
    globals: BTreeMap<usize, OwnedGlobalSpec>,
    table_elements: BTreeMap<usize, TableElement>,
    export_funcs: HashMap<Vec<u8>, *const extern "C" fn()>,
    func_table: HashMap<(u32, u32), *const extern "C" fn()>,
    start_func: Option<extern "C" fn()>,
    trap_manifest: Vec<TrapManifestRecord>,
}

impl MockModuleBuilder {
    pub fn new() -> Self {
        const DEFAULT_HEAP_SPEC: HeapSpec = HeapSpec {
            reserved_size: 4 * 1024 * 1024,
            guard_size: 4 * 1024 * 1024,
            initial_size: 64 * 1024,
            max_size: Some(64 * 1024),
        };
        MockModuleBuilder::default().with_heap_spec(DEFAULT_HEAP_SPEC)
    }

    pub fn with_heap_spec(mut self, heap_spec: HeapSpec) -> Self {
        self.heap_spec = heap_spec;
        self
    }

    pub fn with_initial_heap(mut self, heap: &[u8]) -> Self {
        self.sparse_page_data = heap
            .chunks(4096)
            .map(|page| {
                if page.iter().all(|b| *b == 0) {
                    None
                } else {
                    let mut page = page.to_vec();
                    if page.len() < 4096 {
                        page.resize(4096, 0);
                    }
                    Some(page)
                }
            })
            .collect();
        self
    }

    pub fn with_global(mut self, idx: u32, init_val: i64) -> Self {
        self.globals
            .insert(idx as usize, OwnedGlobalSpec::new_def(init_val, None));
        self
    }

    pub fn with_exported_global(mut self, idx: u32, init_val: i64, export_name: &str) -> Self {
        self.globals.insert(
            idx as usize,
            OwnedGlobalSpec::new_def(init_val, Some(export_name.to_string())),
        );
        self
    }

    pub fn with_import(mut self, idx: u32, import_module: &str, import_field: &str) -> Self {
        self.globals.insert(
            idx as usize,
            OwnedGlobalSpec::new_import(import_module.to_string(), import_field.to_string(), None),
        );
        self
    }

    pub fn with_exported_import(
        mut self,
        idx: u32,
        import_module: &str,
        import_field: &str,
        export_name: &str,
    ) -> Self {
        self.globals.insert(
            idx as usize,
            OwnedGlobalSpec::new_import(
                import_module.to_string(),
                import_field.to_string(),
                Some(export_name.to_string()),
            ),
        );
        self
    }

    pub fn with_table_element(mut self, idx: u32, element: &TableElement) -> Self {
        self.table_elements.insert(idx as usize, element.clone());
        self
    }

    pub fn with_export_func(mut self, sym: &[u8], func: *const extern "C" fn()) -> Self {
        self.export_funcs.insert(sym.to_vec(), func);
        self
    }

    pub fn with_table_func(
        mut self,
        table_idx: u32,
        func_idx: u32,
        func: *const extern "C" fn(),
    ) -> Self {
        self.func_table.insert((table_idx, func_idx), func);
        self
    }

    pub fn with_start_func(mut self, func: extern "C" fn()) -> Self {
        self.start_func = Some(func);
        self
    }

    pub fn with_trap_manifest(mut self, trap_manifest: &[TrapManifestRecord]) -> Self {
        self.trap_manifest = trap_manifest.to_vec();
        self
    }

    pub fn build(self) -> Arc<dyn Module> {
        assert!(
            self.sparse_page_data.len() * 4096 <= self.heap_spec.initial_size as usize,
            "heap must fit in heap spec initial size"
        );

        let table_elements = self
            .table_elements
            .into_iter()
            .enumerate()
            .map(|(expected_idx, (idx, te))| {
                assert_eq!(
                    idx, expected_idx,
                    "table element indices must be contiguous starting from 0"
                );
                te
            })
            .collect();
        let globals_spec = self
            .globals
            .into_iter()
            .enumerate()
            .map(|(expected_idx, (idx, gs))| {
                assert_eq!(
                    idx, expected_idx,
                    "global indices must be contiguous starting from 0"
                );
                gs
            })
            .collect();
        let owned_module_data = OwnedModuleData::new(
            Some(OwnedLinearMemorySpec {
                heap: self.heap_spec,
                initializer: OwnedSparseData::new(self.sparse_page_data)
                    .expect("sparse data pages are valid"),
            }),
            globals_spec,
        );
        let serialized_module_data = owned_module_data
            .to_ref()
            .serialize()
            .expect("serialization of module_data succeeds");
        let module_data = ModuleData::deserialize(&serialized_module_data)
            .map(|md| unsafe { std::mem::transmute(md) })
            .expect("module data can be deserialized");
        let mock = MockModule {
            serialized_module_data,
            module_data,
            table_elements,
            export_funcs: self.export_funcs,
            func_table: self.func_table,
            start_func: self.start_func,
            trap_manifest: self.trap_manifest,
        };
        Arc::new(mock)
    }
}

pub struct MockModule {
    #[allow(dead_code)]
    serialized_module_data: Vec<u8>,
    module_data: ModuleData<'static>,
    pub table_elements: Vec<TableElement>,
    pub export_funcs: HashMap<Vec<u8>, *const extern "C" fn()>,
    pub func_table: HashMap<(u32, u32), *const extern "C" fn()>,
    pub start_func: Option<extern "C" fn()>,
    pub trap_manifest: Vec<TrapManifestRecord>,
}

unsafe impl Send for MockModule {}
unsafe impl Sync for MockModule {}

impl Module for MockModule {}

impl ModuleInternal for MockModule {
    fn heap_spec(&self) -> Option<&HeapSpec> {
        self.module_data.heap_spec()
    }

    fn globals(&self) -> &[GlobalSpec] {
        self.module_data.globals_spec()
    }

    fn get_sparse_page_data(&self, page: usize) -> Option<&[u8]> {
        if let Some(ref sparse_data) = self.module_data.sparse_data() {
            *sparse_data.get_page(page)
        } else {
            None
        }
    }

    fn sparse_page_data_len(&self) -> usize {
        self.module_data.sparse_data().map(|d| d.len()).unwrap_or(0)
    }

    fn table_elements(&self) -> Result<&[TableElement], Error> {
        Ok(&self.table_elements)
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
        // we can call `dladdr` on Rust code, but unless we inspect the stack I don't think there's
        // a way to determine whether or not we're in "module" code; punt for now
        Ok(None)
    }
}
