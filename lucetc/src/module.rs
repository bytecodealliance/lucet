//! Implements ModuleEnvironment for cranelift-wasm. Code derived from cranelift-wasm/environ/dummy.rs
use crate::pointer::NATIVE_POINTER;
use cranelift_codegen::entity::{EntityRef, PrimaryMap};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, ModuleEnvironment, SignatureIndex, Table,
    TableElementType, TableIndex, WasmResult,
};
use lucet_module_data::UniqueSignatureIndex;
use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Clone)]
pub struct Exportable<'a, T> {
    pub entity: T,
    pub export_names: Vec<&'a str>,
}

impl<'a, T> Exportable<'a, T> {
    pub fn new(entity: T) -> Self {
        Self {
            entity,
            export_names: Vec::new(),
        }
    }
    pub fn push_export(&mut self, name: &'a str) {
        self.export_names.push(name);
    }
}

#[derive(Debug, Clone)]
pub struct TableElems {
    pub base: Option<GlobalIndex>,
    pub offset: usize,
    pub elements: Box<[FuncIndex]>,
}

#[derive(Debug, Clone)]
pub struct DataInitializer<'a> {
    pub base: Option<GlobalIndex>,
    pub offset: usize,
    pub data: &'a [u8],
}

pub struct ModuleInfo<'a> {
    /// Target description used for codegen
    pub target_config: TargetFrontendConfig,
    /// This mapping lets us merge duplicate types (permitted by the wasm spec) as they're
    /// declared.
    pub signature_mapping: PrimaryMap<SignatureIndex, UniqueSignatureIndex>,
    /// Provided by `declare_signature`
    pub signatures: PrimaryMap<UniqueSignatureIndex, ir::Signature>,
    /// Provided by `declare_func_import`
    pub imported_funcs: PrimaryMap<FuncIndex, (&'a str, &'a str)>,
    /// Provided by `declare_global_import`
    pub imported_globals: PrimaryMap<GlobalIndex, (&'a str, &'a str)>,
    /// Provided by `declare_table_import`
    pub imported_tables: PrimaryMap<TableIndex, (&'a str, &'a str)>,
    /// Provided by `declare_memory_import`
    pub imported_memories: PrimaryMap<MemoryIndex, (&'a str, &'a str)>,
    /// Function signatures: imported and local
    pub functions: PrimaryMap<FuncIndex, Exportable<'a, SignatureIndex>>,
    /// Provided by `declare_table`
    pub tables: PrimaryMap<TableIndex, Exportable<'a, Table>>,
    /// Provided by `declare_memory`
    pub memories: PrimaryMap<MemoryIndex, Exportable<'a, Memory>>,
    /// Provided by `declare_global`
    pub globals: PrimaryMap<GlobalIndex, Exportable<'a, Global>>,
    /// Provided by `declare_start_func`
    pub start_func: Option<FuncIndex>,

    /// Function bodies: local only
    pub function_bodies: HashMap<FuncIndex, (&'a [u8], usize)>,

    /// Table elements: local only
    pub table_elems: HashMap<TableIndex, Vec<TableElems>>,

    /// Data initializers: local only
    pub data_initializers: HashMap<MemoryIndex, Vec<DataInitializer<'a>>>,
}

impl<'a> ModuleInfo<'a> {
    pub fn new(target_config: TargetFrontendConfig) -> Self {
        Self {
            target_config,
            signature_mapping: PrimaryMap::new(),
            signatures: PrimaryMap::new(),
            imported_funcs: PrimaryMap::new(),
            imported_globals: PrimaryMap::new(),
            imported_tables: PrimaryMap::new(),
            imported_memories: PrimaryMap::new(),
            functions: PrimaryMap::new(),
            tables: PrimaryMap::new(),
            memories: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            start_func: None,
            function_bodies: HashMap::new(),
            table_elems: HashMap::new(),
            data_initializers: HashMap::new(),
        }
    }

    pub fn signature_for_function(&self, func_index: FuncIndex) -> &ir::Signature {
        // FuncIndex are valid (or the caller has very bad data)
        let sigidx = self.functions.get(func_index).unwrap().entity;

        self.signature_by_id(sigidx)
    }

    pub fn signature_by_id(&self, sig_idx: SignatureIndex) -> &ir::Signature {
        // All signatures map to some unique signature index
        let unique_sig_idx = self.signature_mapping.get(sig_idx).unwrap();
        // Unique signature indices are valid (or we're in some deeply bad state)
        self.signatures.get(*unique_sig_idx).unwrap()
    }

    pub fn declare_func_with_sig(&mut self, sig: ir::Signature) -> (FuncIndex, SignatureIndex) {
        let new_sigidx = SignatureIndex::from_u32(self.signature_mapping.len() as u32);
        self.declare_signature(sig);
        let new_funcidx = FuncIndex::from_u32(self.functions.len() as u32);
        self.declare_func_type(new_sigidx);
        (new_funcidx, new_sigidx)
    }
}

impl<'a> ModuleEnvironment<'a> for ModuleInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.target_config
    }

    fn declare_signature(&mut self, mut sig: ir::Signature) {
        sig.params.insert(
            0,
            ir::AbiParam::special(NATIVE_POINTER, ir::ArgumentPurpose::VMContext),
        );

        let match_key = self
            .signatures
            .iter()
            .find(|(_, v)| *v == &sig)
            .map(|(key, _)| key)
            .unwrap_or_else(|| {
                let lucet_sig_ix = UniqueSignatureIndex::from_u32(self.signatures.len() as u32);
                self.signatures.push(sig);
                lucet_sig_ix
            });

        self.signature_mapping.push(match_key);
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &'a str, field: &'a str) {
        debug_assert_eq!(
            self.functions.len(),
            self.imported_funcs.len(),
            "import functions are declared first"
        );
        self.functions.push(Exportable::new(sig_index));
        self.imported_funcs.push((module, field));
    }

    fn declare_global_import(&mut self, global: Global, module: &'a str, field: &'a str) {
        debug_assert_eq!(
            self.globals.len(),
            self.imported_globals.len(),
            "import globals are declared first"
        );
        self.globals.push(Exportable::new(global));
        self.imported_globals.push((module, field));
    }

    fn declare_table_import(&mut self, table: Table, module: &'a str, field: &'a str) {
        debug_assert_eq!(
            self.tables.len(),
            self.imported_tables.len(),
            "import tables are declared first"
        );
        self.tables.push(Exportable::new(table));
        self.imported_tables.push((module, field));
    }

    fn declare_memory_import(&mut self, memory: Memory, module: &'a str, field: &'a str) {
        debug_assert_eq!(
            self.memories.len(),
            self.imported_memories.len(),
            "import memories are declared first"
        );
        self.data_initializers
            .insert(MemoryIndex::new(self.memories.len()), vec![]);
        self.memories.push(Exportable::new(memory));
        self.imported_memories.push((module, field));
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.functions.push(Exportable::new(sig_index));
    }

    fn declare_table(&mut self, table: Table) {
        self.table_elems
            .insert(TableIndex::new(self.tables.len()), vec![]);
        self.tables.push(Exportable::new(table));
    }

    fn declare_memory(&mut self, memory: Memory) {
        self.data_initializers
            .insert(MemoryIndex::new(self.memories.len()), vec![]);
        self.memories.push(Exportable::new(memory));
    }

    fn declare_global(&mut self, global: Global) {
        self.globals.push(Exportable::new(global));
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &'a str) {
        self.functions
            .get_mut(func_index)
            .expect("export of declared function")
            .push_export(name);
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &'a str) {
        self.tables
            .get_mut(table_index)
            .expect("export of declared table")
            .push_export(name);
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &'a str) {
        self.memories
            .get_mut(memory_index)
            .expect("export of declared memory")
            .push_export(name);
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &'a str) {
        self.globals
            .get_mut(global_index)
            .expect("export of declared global")
            .push_export(name);
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) {
        debug_assert!(
            self.start_func.is_none(),
            "start func can only be defined once"
        );
        self.start_func = Some(func_index);
    }

    fn define_function_body(&mut self, body_bytes: &'a [u8], body_offset: usize) -> WasmResult<()> {
        let func_index = FuncIndex::new(self.imported_funcs.len() + self.function_bodies.len());
        self.function_bodies
            .insert(func_index, (body_bytes, body_offset));
        Ok(())
    }

    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        elements: Box<[FuncIndex]>,
    ) {
        let table_elems = TableElems {
            base,
            offset,
            elements,
        };
        match self.table_elems.entry(table_index) {
            Entry::Occupied(mut occ) => occ.get_mut().push(table_elems),
            Entry::Vacant(vac) => {
                if self.tables.len() == 0 && table_index == TableIndex::new(0) {
                    let table = Table {
                        ty: TableElementType::Func,
                        minimum: 0,
                        maximum: None,
                    };
                    self.tables.push(Exportable::new(table));
                    vac.insert(vec![table_elems]);
                } else {
                    panic!("creation of elements for undeclared table! only table 0 is implicitly declared") // Do we implicitly declare them all???? i sure hope not
                }
            }
        }
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'a [u8],
    ) {
        let data_init = DataInitializer { base, offset, data };
        match self.data_initializers.entry(memory_index) {
            Entry::Occupied(mut occ) => occ.get_mut().push(data_init),
            Entry::Vacant(_) => panic!(
                "data initializer for undeclared memory {:?}: {:?}",
                memory_index, data_init
            ),
        }
    }
}
