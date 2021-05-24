//! Implements ModuleEnvironment for cranelift-wasm. Code derived from cranelift-wasm/environ/dummy.rs
use crate::error::Error;
use crate::pointer::NATIVE_POINTER;
use crate::validate::Validator;
use cranelift_codegen::entity::{entity_impl, EntityRef, PrimaryMap, SecondaryMap};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    wasmparser::{FuncValidator, FunctionBody, ValidatorResources},
    DataIndex, ElemIndex, FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, ModuleEnvironment,
    Table, TableElementType, TableIndex, TargetEnvironment, TypeIndex, WasmError, WasmFuncType,
    WasmResult, WasmType,
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
    pub offset: u32,
    pub elements: Box<[UniqueFuncIndex]>,
}

#[derive(Debug, Clone)]
pub struct DataInitializer<'a> {
    pub base: Option<GlobalIndex>,
    pub offset: u32,
    pub data: &'a [u8],
}

pub struct ModuleInfo<'a> {
    /// Target description used for codegen
    pub target_config: TargetFrontendConfig,
    /// This mapping lets us merge duplicate types (permitted by the wasm spec) as they're
    /// declared.
    pub signature_mapping: PrimaryMap<TypeIndex, UniqueSignatureIndex>,
    /// Provided by `declare_type_func`
    pub signatures: PrimaryMap<UniqueSignatureIndex, (ir::Signature, WasmFuncType)>,
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
    pub functions: PrimaryMap<UniqueFuncIndex, Exportable<'a, TypeIndex>>,
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

    /// Table elements: local only
    pub table_elems: HashMap<TableIndex, Vec<TableElems>>,

    /// Data initializers: local only
    pub data_initializers: HashMap<MemoryIndex, Vec<DataInitializer<'a>>>,
}

pub struct ModuleValidation<'a> {
    /// Witx validator
    pub validator: Option<Validator>,
    /// Module IR:
    pub info: ModuleInfo<'a>,
    /// Function bodies: local only
    pub function_bodies:
        HashMap<UniqueFuncIndex, (FuncValidator<ValidatorResources>, FunctionBody<'a>)>,
}

impl<'a> ModuleValidation<'a> {
    pub fn new(target_config: TargetFrontendConfig, validator: Option<Validator>) -> Self {
        Self {
            validator,
            info: ModuleInfo::new(target_config),
            function_bodies: HashMap::new(),
        }
    }

    pub fn validation_errors(&self) -> Result<(), Error> {
        if let Some(ref v) = self.validator {
            v.report().map_err(Error::LucetValidation)
        } else {
            Ok(())
        }
    }
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
            table_elems: HashMap::new(),
            data_initializers: HashMap::new(),
        }
    }

    pub fn signature_for_function(
        &self,
        func_index: UniqueFuncIndex,
    ) -> &(ir::Signature, WasmFuncType) {
        // UniqueFuncIndex are valid (or the caller has very bad data)
        let sigidx = self.functions.get(func_index).unwrap().entity;

        self.signature_by_id(sigidx)
    }

    pub fn signature_by_id(&self, sig_idx: TypeIndex) -> &(ir::Signature, WasmFuncType) {
        // All signatures map to some unique signature index
        let unique_sig_idx = self.signature_mapping.get(sig_idx).unwrap();
        // Unique signature indices are valid (or we're in some deeply bad state)
        self.signatures.get(*unique_sig_idx).unwrap()
    }

    pub fn declare_func_with_sig(
        &mut self,
        wasm_func_type: WasmFuncType,
        sig: ir::Signature,
    ) -> Result<(UniqueFuncIndex, TypeIndex), Error> {
        let new_sigidx = TypeIndex::from_u32(self.signature_mapping.len() as u32);
        self.declare_type_func(wasm_func_type, sig)?;
        let new_funcidx = UniqueFuncIndex::from_u32(self.functions.len() as u32);
        self.declare_func_type(new_sigidx)?;
        Ok((new_funcidx, new_sigidx))
    }

    fn declare_type_func(
        &mut self,
        wasm_func_type: WasmFuncType,
        mut sig: ir::Signature,
    ) -> WasmResult<()> {
        sig.params.insert(
            0,
            ir::AbiParam::special(NATIVE_POINTER, ir::ArgumentPurpose::VMContext),
        );

        let match_key = self
            .signatures
            .iter()
            .find(|(_, (ssig, _))| ssig == &sig)
            .map(|(key, _)| key)
            .unwrap_or_else(|| {
                let lucet_sig_ix = UniqueSignatureIndex::from_u32(self.signatures.len() as u32);
                self.signatures.push((sig, wasm_func_type.clone()));
                lucet_sig_ix
            });

        self.signature_mapping.push(match_key);
        Ok(())
    }
    fn declare_func_type(&mut self, sig_index: TypeIndex) -> WasmResult<()> {
        self.functions.push(Exportable::new(sig_index));
        self.function_mapping
            .push(UniqueFuncIndex::from_u32(self.functions.len() as u32 - 1));
        Ok(())
    }
}

impl<'a> TargetEnvironment for ModuleInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.target_config
    }
}

impl<'a> TargetEnvironment for ModuleValidation<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.info.target_config()
    }
}

impl<'a> ModuleEnvironment<'a> for ModuleValidation<'a> {
    fn declare_type_func(&mut self, wasm: WasmFuncType) -> WasmResult<()> {
        let mut sig = ir::Signature::new(self.info.target_config.default_call_conv);
        let cvt = |ty: &WasmType| {
            ir::AbiParam::new(match ty {
                WasmType::I32 => ir::types::I32,
                WasmType::I64 => ir::types::I64,
                WasmType::F32 => ir::types::F32,
                WasmType::F64 => ir::types::F64,
                _ => unimplemented!(),
            })
        };
        sig.params.extend(wasm.params.iter().map(&cvt));
        sig.returns.extend(wasm.returns.iter().map(&cvt));
        self.info.declare_type_func(wasm, sig)
    }
    fn declare_func_import(
        &mut self,
        sig_index: TypeIndex,
        module: &'a str,
        field: Option<&'a str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.info.functions.len(),
            self.info.imported_funcs.len(),
            "import functions are declared first"
        );

        let field = field.expect("lucet does not support optional `field` in  imports");
        let unique_fn_index = self
            .info
            .imported_funcs
            .iter()
            .find(|(_, v)| *v == &(module, field))
            .map(|(key, _)| key)
            .unwrap_or_else(|| {
                self.info.functions.push(Exportable::new(sig_index));
                self.info.imported_funcs.push((module, field));
                UniqueFuncIndex::from_u32(self.info.functions.len() as u32 - 1)
            });

        self.info.function_mapping.push(unique_fn_index);

        let (_sig, func_type) = self.info.signature_by_id(sig_index).clone();
        if let Some(ref mut v) = self.validator {
            v.register_import(module, field, &func_type);
        }
        Ok(())
    }

    fn declare_global_import(
        &mut self,
        global: Global,
        module: &'a str,
        field: Option<&'a str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.info.globals.len(),
            self.info.imported_globals.len(),
            "import globals are declared first"
        );

        let field = field.expect("lucet does not support optional `field` in  imports");
        self.info.globals.push(Exportable::new(global));
        self.info.imported_globals.push((module, field));
        Ok(())
    }

    fn declare_table_import(
        &mut self,
        table: Table,
        module: &'a str,
        field: Option<&'a str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.info.tables.len(),
            self.info.imported_tables.len(),
            "import tables are declared first"
        );

        let field = field.expect("lucet does not support optional `field` in  imports");
        self.info.tables.push(Exportable::new(table));
        self.info.imported_tables.push((module, field));
        Ok(())
    }

    fn declare_memory_import(
        &mut self,
        memory: Memory,
        module: &'a str,
        field: Option<&'a str>,
    ) -> WasmResult<()> {
        debug_assert_eq!(
            self.info.memories.len(),
            self.info.imported_memories.len(),
            "import memories are declared first"
        );

        let field = field.expect("lucet does not support optional `field` in  imports");
        self.info
            .data_initializers
            .insert(MemoryIndex::new(self.info.memories.len()), vec![]);
        self.info.memories.push(Exportable::new(memory));
        self.info.imported_memories.push((module, field));
        Ok(())
    }

    fn declare_func_type(&mut self, sig_index: TypeIndex) -> WasmResult<()> {
        self.info.declare_func_type(sig_index)
    }

    fn declare_table(&mut self, table: Table) -> WasmResult<()> {
        self.info
            .table_elems
            .insert(TableIndex::new(self.info.tables.len()), vec![]);
        self.info.tables.push(Exportable::new(table));
        Ok(())
    }

    fn declare_memory(&mut self, memory: Memory) -> WasmResult<()> {
        self.info
            .data_initializers
            .insert(MemoryIndex::new(self.info.memories.len()), vec![]);
        self.info.memories.push(Exportable::new(memory));
        Ok(())
    }

    fn declare_global(&mut self, global: Global) -> WasmResult<()> {
        self.info.globals.push(Exportable::new(global));
        Ok(())
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &'a str) -> WasmResult<()> {
        let unique_func_index = *self
            .info
            .function_mapping
            .get(func_index)
            .expect("function indices are valid");
        self.info
            .functions
            .get_mut(unique_func_index)
            .expect("export of declared function")
            .push_export(name);

        let (_sig, func_type) = self.info.signature_for_function(unique_func_index).clone();
        if let Some(ref mut v) = self.validator {
            v.register_export(name, &func_type)
        }
        Ok(())
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &'a str) -> WasmResult<()> {
        self.info
            .tables
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
        self.info
            .memories
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
        self.info
            .globals
            .get_mut(global_index)
            .expect("export of declared global")
            .push_export(name);
        Ok(())
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) -> WasmResult<()> {
        let unique_func_index = *self
            .info
            .function_mapping
            .get(func_index)
            .expect("function indices are valid");
        debug_assert!(
            self.info.start_func.is_none(),
            "start func can only be defined once"
        );
        self.info.start_func = Some(unique_func_index);
        Ok(())
    }

    fn define_function_body(
        &mut self,
        func_validator: FuncValidator<ValidatorResources>,
        body: FunctionBody<'a>,
    ) -> WasmResult<()> {
        let func_index =
            UniqueFuncIndex::new(self.info.imported_funcs.len() + self.function_bodies.len());
        self.function_bodies
            .insert(func_index, (func_validator, body));
        Ok(())
    }

    fn declare_table_elements(
        &mut self,
        table_index: TableIndex,
        base: Option<GlobalIndex>,
        offset: u32,
        elements: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        let elements_vec: Vec<FuncIndex> = elements.into();
        let uniquified_elements = elements_vec
            .into_iter()
            .map(|fn_idx| {
                *self
                    .info
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
        match self.info.table_elems.entry(table_index) {
            Entry::Occupied(mut occ) => occ.get_mut().push(table_elems),
            Entry::Vacant(vac) => {
                if self.info.tables.is_empty() && table_index == TableIndex::new(0) {
                    let table = Table {
                        ty: TableElementType::Func,
                        wasm_ty: WasmType::FuncRef,
                        minimum: 0,
                        maximum: None,
                    };
                    self.info.tables.push(Exportable::new(table));
                    vac.insert(vec![table_elems]);
                } else {
                    return Err(WasmError::User("creation of elements for undeclared table! only table 0 is implicitly declared".to_string()));
                }
            }
        }
        Ok(())
    }

    fn declare_data_initialization(
        &mut self,
        memory_index: MemoryIndex,
        base: Option<GlobalIndex>,
        offset: u32,
        data: &'a [u8],
    ) -> WasmResult<()> {
        let data_init = DataInitializer { base, offset, data };
        match self.info.data_initializers.entry(memory_index) {
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

    fn declare_func_name(&mut self, func_index: FuncIndex, name: &'a str) {
        let unique_func_index = *self
            .info
            .function_mapping
            .get(func_index)
            .expect("function indices are valid");
        self.info.function_names[unique_func_index] = name;
    }

    fn declare_passive_element(
        &mut self,
        _index: ElemIndex,
        _elements: Box<[FuncIndex]>,
    ) -> WasmResult<()> {
        unimplemented!();
    }

    fn declare_passive_data(&mut self, _data_index: DataIndex, _data: &'a [u8]) -> WasmResult<()> {
        unimplemented!();
    }
}
