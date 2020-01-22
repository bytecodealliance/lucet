//! Implements ModuleEnvironment for cranelift-wasm. Code derived from cranelift-wasm/environ/dummy.rs
use crate::error::Error;
use crate::pointer::NATIVE_POINTER;
use cranelift_codegen::entity::{entity_impl, EntityRef, PrimaryMap, SecondaryMap};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, ModuleEnvironment, ModuleTranslationState,
    SignatureIndex, Table, TableElementType, TableIndex, WasmResult,
};
use lucet_module::UniqueSignatureIndex;
use std::collections::{hash_map::Entry, HashMap};

/// UniqueFuncIndex names a function after merging duplicate function declarations to a single
/// identifier, whereas FuncIndex is maintained by Cranelift and may have multiple indices referring
/// to a single function in the resulting artifact.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct UniqueFuncIndex(u32);
entity_impl!(UniqueFuncIndex);

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
    pub elements: Box<[UniqueFuncIndex]>,
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
    pub imported_funcs: PrimaryMap<UniqueFuncIndex, (&'a str, &'a str)>,
    /// Provided by `declare_global_import`
    pub imported_globals: PrimaryMap<GlobalIndex, (&'a str, &'a str)>,
    /// Provided by `declare_table_import`
    pub imported_tables: PrimaryMap<TableIndex, (&'a str, &'a str)>,
    /// Provided by `declare_memory_import`
    pub imported_memories: PrimaryMap<MemoryIndex, (&'a str, &'a str)>,
    /// This mapping lets us merge duplicate functions (for example, multiple import declarations)
    /// as they're declared.
    pub function_mapping: PrimaryMap<FuncIndex, UniqueFuncIndex>,
    /// Function signatures: imported and local
    pub functions: PrimaryMap<UniqueFuncIndex, Exportable<'a, SignatureIndex>>,
    /// Function names.
    pub function_names: SecondaryMap<UniqueFuncIndex, &'a str>,
    /// Provided by `declare_table`
    pub tables: PrimaryMap<TableIndex, Exportable<'a, Table>>,
    /// Provided by `declare_memory`
    pub memories: PrimaryMap<MemoryIndex, Exportable<'a, Memory>>,
    /// Provided by `declare_global`
    pub globals: PrimaryMap<GlobalIndex, Exportable<'a, Global>>,
    /// Provided by `declare_start_func`
    pub start_func: Option<UniqueFuncIndex>,

    /// Function bodies: local only
    pub function_bodies: HashMap<UniqueFuncIndex, (&'a [u8], usize)>,

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
            function_mapping: PrimaryMap::new(),
            functions: PrimaryMap::new(),
            function_names: SecondaryMap::new(),
            tables: PrimaryMap::new(),
            memories: PrimaryMap::new(),
            globals: PrimaryMap::new(),
            start_func: None,
            function_bodies: HashMap::new(),
            table_elems: HashMap::new(),
            data_initializers: HashMap::new(),
        }
    }

    pub fn signature_for_function(&self, func_index: UniqueFuncIndex) -> &ir::Signature {
        // UniqueFuncIndex are valid (or the caller has very bad data)
        let sigidx = self.functions.get(func_index).unwrap().entity;

        self.signature_by_id(sigidx)
    }

    pub fn signature_by_id(&self, sig_idx: SignatureIndex) -> &ir::Signature {
        // All signatures map to some unique signature index
        let unique_sig_idx = self.signature_mapping.get(sig_idx).unwrap();
        // Unique signature indices are valid (or we're in some deeply bad state)
        self.signatures.get(*unique_sig_idx).unwrap()
    }

    pub fn declare_func_with_sig(
        &mut self,
        sig: ir::Signature,
    ) -> Result<(UniqueFuncIndex, SignatureIndex), Error> {
        let new_sigidx = SignatureIndex::from_u32(self.signature_mapping.len() as u32);
        self.declare_signature(sig)?;
        let new_funcidx = UniqueFuncIndex::from_u32(self.functions.len() as u32);
        self.declare_func_type(new_sigidx)?;
        Ok((new_funcidx, new_sigidx))
    }
}

impl<'a> ModuleEnvironment<'a> for ModuleInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.target_config
    }

    fn declare_signature(&mut self, mut sig: ir::Signature) -> WasmResult<()> {
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
        Ok(())
    }

    fn declare_func_import(
        &mut self,
        sig_index: SignatureIndex,
        module: &'a str,
        field: &'a str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.functions.len(),
            self.imported_funcs.len(),
            "import functions are declared first"
        );

        let unique_fn_index = self
            .imported_funcs
            .iter()
            .find(|(_, v)| *v == &(module, field))
            .map(|(key, _)| key)
            .unwrap_or_else(|| {
                self.functions.push(Exportable::new(sig_index));
                self.imported_funcs.push((module, field));
                UniqueFuncIndex::from_u32(self.functions.len() as u32 - 1)
            });

        self.function_mapping.push(unique_fn_index);
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &'a str,
        field: &'a str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.globals.len(),
            self.imported_globals.len(),
            "import globals are declared first"
        );
        self.globals.push(Exportable::new(global));
        self.imported_globals.push((module, field));
        Ok(())
    }

    fn declare_table_import(
        &mut self,
        table: Table,
        module: &'a str,
        field: &'a str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.tables.len(),
            self.imported_tables.len(),
            "import tables are declared first"
        );
        self.tables.push(Exportable::new(table));
        self.imported_tables.push((module, field));
        Ok(())
    }

    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &'a str,
        field: &'a str,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.memories.len(),
            self.imported_memories.len(),
            "import memories are declared first"
        );
        self.data_initializers
            .insert(MemoryIndex::new(self.memories.len()), vec![]);
        self.memories.push(Exportable::new(memory));
        self.imported_memories.push((module, field));
        Ok(())
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) -> WasmResult<()> {
        self.functions.push(Exportable::new(sig_index));
        self.function_mapping
            .push(UniqueFuncIndex::from_u32(self.functions.len() as u32 - 1));
        Ok(())
    }

    fn declare_table(&mut self, table: Table) -> WasmResult<()> {
        self.table_elems
            .insert(TableIndex::new(self.tables.len()), vec![]);
        self.tables.push(Exportable::new(table));
        Ok(())
    }

    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()> {
        self.data_initializers
            .insert(MemoryIndex::new(self.memories.len()), vec![]);
        self.memories.push(Exportable::new(memory));
        Ok(())
    }

    fn declare_global(&mut self, global: Global) -> WasmResult<()> {
        self.globals.push(Exportable::new(global));
        Ok(())
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &'a str) -> WasmResult<()> {
        let unique_func_index = *self
            .function_mapping
            .get(func_index)
            .expect("function indices are valid");
        self.functions
            .get_mut(unique_func_index)
            .expect("export of declared function")
            .push_export(name);
        Ok(())
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &'a str) -> WasmResult<()> {
        self.tables
            .get_mut(table_index)
            .expect("export of declared table")
            .push_export(name);
        Ok(())
    }

    fn declare_memory_export(
        &mut self,
        memory_index: MemoryIndex,
        name: &'a str,
    ) -> WasmResult<()> {
        self.memories
            .get_mut(memory_index)
            .expect("export of declared memory")
            .push_export(name);
        Ok(())
    }

    fn declare_global_export(
        &mut self,
        global_index: GlobalIndex,
        name: &'a str,
    ) -> WasmResult<()> {
        self.globals
            .get_mut(global_index)
            .expect("export of declared global")
            .push_export(name);
        Ok(())
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) -> WasmResult<()> {
        let unique_func_index = *self
            .function_mapping
            .get(func_index)
            .expect("function indices are valid");
        debug_assert!(
            self.start_func.is_none(),
            "start func can only be defined once"
        );
        self.start_func = Some(unique_func_index);
        Ok(())
    }

    fn define_function_body(
        &mut self,
        _module_translation_state: &ModuleTranslationState,
        body_bytes: &'a [u8],
        body_offset: usize,
    ) -> WasmResult<()> {
        let func_index =
            UniqueFuncIndex::new(self.imported_funcs.len() + self.function_bodies.len());
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
    ) -> WasmResult<()> {
        let elements_vec: Vec<FuncIndex> = elements.into();
        let uniquified_elements = elements_vec
            .into_iter()
            .map(|fn_idx| {
                *self
                    .function_mapping
                    .get(fn_idx)
                    .expect("function indices are valid")
            })
            .collect();
        let table_elems = TableElems {
            base,
            offset,
            elements: uniquified_elements,
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
                    panic!("creation of elements for undeclared table! only table 0 is implicitly declared")
                    // Do we implicitly declare them all???? i sure hope not
                }
            }
        }
        Ok(())
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: usize,
        data: &'a [u8],
    ) -> WasmResult<()> {
        let data_init = DataInitializer { base, offset, data };
        match self.data_initializers.entry(memory_index) {
            Entry::Occupied(mut occ) => {
                occ.get_mut().push(data_init);
            }
            Entry::Vacant(_) => panic!(
                "data initializer for undeclared memory {:?}: {:?}",
                memory_index, data_init
            ),
        }
        Ok(())
    }

    fn declare_func_name(&mut self, func_index: FuncIndex, name: &'a str) -> WasmResult<()> {
        let unique_func_index = *self
            .function_mapping
            .get(func_index)
            .expect("function indices are valid");
        self.function_names[unique_func_index] = name;
        Ok(())
    }
}
