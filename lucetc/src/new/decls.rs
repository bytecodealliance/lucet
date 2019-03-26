use crate::bindings::Bindings;
use crate::compiler::name::Name;
use crate::error::{LucetcError, LucetcErrorKind};
pub use crate::new::module::Exportable;
use crate::new::module::{ModuleInfo, DataInitializer};
use crate::new::runtime::{Runtime, RuntimeFunc};
use crate::program::memory::HeapSettings;
use cranelift_codegen::entity::{EntityRef, PrimaryMap};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_module::{Backend as ClifBackend, Linkage, Module as ClifModule};
use cranelift_wasm::{
    FuncIndex, Global, GlobalIndex, MemoryIndex, ModuleEnvironment,
    SignatureIndex, Table, TableIndex,
};
use failure::{format_err, Error, ResultExt};
use lucet_module_data::HeapSpec;
use std::collections::HashMap;

pub struct FunctionDecl<'a> {
    pub import_name: Option<(&'a str, &'a str)>,
    pub export_names: Vec<&'a str>,
    pub signature: &'a ir::Signature,
    pub name: Name,
}

impl<'a> FunctionDecl<'a> {
    pub fn defined(&self) -> bool {
        self.import_name.is_none()
    }
    pub fn imported(&self) -> bool {
        !self.defined()
    }
}

pub struct RuntimeDecl<'a> {
    pub signature: &'a ir::Signature,
    pub name: Name,
}

pub struct TableDecl<'a> {
    pub import_name: Option<(&'a str, &'a str)>,
    pub export_names: Vec<&'a str>,
    pub table: &'a Table,
    pub contents_name: Name,
    pub len_name: Name,
}

pub struct ModuleDecls<'a> {
    info: ModuleInfo<'a>,
    runtime: Runtime,
    function_names: PrimaryMap<FuncIndex, Name>,
    table_names: PrimaryMap<TableIndex, (Name, Name)>,
    runtime_names: HashMap<RuntimeFunc, Name>,
    heaps: PrimaryMap<MemoryIndex, HeapSpec>,
}

impl<'a> ModuleDecls<'a> {
    pub fn declare<B: ClifBackend>(
        info: ModuleInfo<'a>,
        clif_module: &mut ClifModule<B>,
        bindings: &Bindings,
        runtime: Runtime,
        heap_settings: HeapSettings,
    ) -> Result<Self, LucetcError> {
        let function_names = Self::declare_funcs(&info, clif_module, bindings)?;
        let table_names = Self::declare_tables(&info, clif_module)?;
        let runtime_names = Self::declare_runtime(&runtime, clif_module)?;
        let heaps = Self::declare_heaps(&info, heap_settings)?;
        Ok(Self {
            info,
            function_names,
            table_names,
            runtime_names,
            runtime,
            heaps,
        })
    }

    fn declare_funcs<B: ClifBackend>(
        info: &ModuleInfo<'a>,
        clif_module: &mut ClifModule<B>,
        bindings: &Bindings,
    ) -> Result<PrimaryMap<FuncIndex, Name>, LucetcError> {
        let mut function_names = PrimaryMap::new();
        for ix in 0..info.functions.len() {
            let func_index = FuncIndex::new(ix);
            let exportable_sigix = info.functions.get(func_index).unwrap();
            let signature = info.signatures.get(exportable_sigix.entity).unwrap();
            let name = if let Some((import_mod, import_field)) = info.imported_funcs.get(func_index)
            {
                let import_symbol = bindings
                    .translate(import_mod, import_field)
                    .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
                let funcid = clif_module
                    .declare_function(&import_symbol, Linkage::Import, signature)
                    .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
                Name::new_func(import_symbol, funcid)
            } else {
                if exportable_sigix.export_names.is_empty() {
                    let def_symbol = format!("guest_func_{}", ix);
                    let funcid = clif_module
                        .declare_function(&def_symbol, Linkage::Local, signature)
                        .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
                    Name::new_func(def_symbol, funcid)
                } else {
                    let export_symbol = format!("guest_func_{}", exportable_sigix.export_names[0]);
                    let funcid = clif_module
                        .declare_function(&export_symbol, Linkage::Export, signature)
                        .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
                    Name::new_func(export_symbol, funcid)
                }
            };
            function_names.push(name);
        }
        Ok(function_names)
    }

    fn declare_tables<B: ClifBackend>(
        info: &ModuleInfo<'a>,
        clif_module: &mut ClifModule<B>,
    ) -> Result<PrimaryMap<TableIndex, (Name, Name)>, LucetcError> {
        let mut table_names = PrimaryMap::new();
        for ix in 0..info.tables.len() {
            let def_symbol = format!("guest_table_{}", ix);
            let def_data_id = clif_module
                .declare_data(&def_symbol, Linkage::Local, false)
                .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
            let def_name = Name::new_data(def_symbol, def_data_id);

            let len_symbol = format!("guest_table_{}_len", ix);
            let len_data_id = clif_module
                .declare_data(&len_symbol, Linkage::Local, false)
                .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
            let len_name = Name::new_data(len_symbol, len_data_id);

            table_names.push((def_name, len_name));
        }
        Ok(table_names)
    }

    fn declare_heaps(
        info: &ModuleInfo<'a>,
        heap_settings: HeapSettings,
    ) -> Result<PrimaryMap<MemoryIndex, HeapSpec>, LucetcError> {
        let mut heaps = PrimaryMap::new();

        for ix in 0..info.memories.len() {
            let ix = MemoryIndex::new(ix);

            if ix != MemoryIndex::new(0) {
                Err(format_err!("lucetc only supports memory 0"))?
            }

            let memory = info.memories.get(ix).expect("memory in range").entity;

            let wasm_page: u64 = 64 * 1024;
            let initial_size = memory.minimum as u64 * wasm_page;
            // Find the max size permitted by the heap and the memory spec
            let max_size = memory.maximum.map(|pages| pages as u64 * wasm_page);
            heaps.push(HeapSpec {
                reserved_size: heap_settings.reserved_size,
                guard_size: heap_settings.guard_size,
                initial_size: initial_size,
                max_size: max_size,
            });
        }
        Ok(heaps)
    }

    fn declare_runtime<B: ClifBackend>(
        runtime: &Runtime,
        clif_module: &mut ClifModule<B>,
    ) -> Result<HashMap<RuntimeFunc, Name>, LucetcError> {
        let mut runtime_names: HashMap<RuntimeFunc, Name> = HashMap::new();
        for (func, (symbol, signature)) in runtime.functions.iter() {
            let funcid = clif_module
                .declare_function(&symbol, Linkage::Import, signature)
                .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
            let name = Name::new_func(symbol.clone(), funcid);

            runtime_names.insert(*func, name);
        }
        Ok(runtime_names)
    }

    pub fn target_config(&self) -> TargetFrontendConfig {
        self.info.target_config()
    }

    pub fn function_bodies(&self) -> impl Iterator<Item = (FunctionDecl, &(&'a [u8], usize))> {
        Box::new(
            self.info
                .function_bodies
                .iter()
                .map(move |(fidx, code)| (self.get_func(*fidx).unwrap(), code)),
        )
    }

    pub fn get_func(&self, func_index: FuncIndex) -> Result<FunctionDecl, Error> {
        let name = self
            .function_names
            .get(func_index)
            .ok_or_else(|| format_err!("func index out of bounds: {:?}", func_index))?;
        let exportable_sigix = self.info.functions.get(func_index).unwrap();
        let signature = self.info.signatures.get(exportable_sigix.entity).unwrap();
        let import_name = self.info.imported_funcs.get(func_index);
        Ok(FunctionDecl {
            signature,
            export_names: exportable_sigix.export_names.clone(),
            import_name: import_name.cloned(),
            name: name.clone(),
        })
    }

    pub fn get_runtime(&self, runtime_func: RuntimeFunc) -> Result<RuntimeDecl, Error> {
        let (_, signature) = self
            .runtime
            .functions
            .get(&runtime_func)
            .ok_or_else(|| format_err!("runtime func not supported: {:?}", runtime_func))?;
        let name = self.runtime_names.get(&runtime_func).unwrap();
        Ok(RuntimeDecl {
            signature,
            name: name.clone(),
        })
    }

    pub fn get_table(&self, table_index: TableIndex) -> Result<TableDecl, Error> {
        let (contents_name, len_name) = self
            .table_names
            .get(table_index)
            .ok_or_else(|| format_err!("table index out of bounds: {:?}", table_index))?;
        let exportable_tbl = self.info.tables.get(table_index).unwrap();
        let import_name = self.info.imported_tables.get(table_index);
        Ok(TableDecl {
            table: &exportable_tbl.entity,
            export_names: exportable_tbl.export_names.clone(),
            import_name: import_name.cloned(),
            contents_name: contents_name.clone(),
            len_name: len_name.clone(),
        })
    }

    pub fn get_signature(&self, signature_index: SignatureIndex) -> Result<&ir::Signature, Error> {
        self.info
            .signatures
            .get(signature_index)
            .ok_or_else(|| format_err!("signature out of bounds: {:?}", signature_index))
    }

    pub fn get_global(&self, global_index: GlobalIndex) -> Result<&Exportable<Global>, Error> {
        self.info
            .globals
            .get(global_index)
            .ok_or_else(|| format_err!("global out of bounds: {:?}", global_index))
    }

    pub fn get_heap(&self, mem_index: MemoryIndex) -> Result<&HeapSpec, Error> {
        self.heaps
            .get(mem_index)
            .ok_or_else(|| format_err!("linear memory out of bounds: {:?}", mem_index))
    }

    pub fn get_data_initializers(
        &self,
        mem_index: MemoryIndex,
    ) -> Result<&[DataInitializer<'a>], Error> {
        self.info
            .data_initializers
            .get(&mem_index)
            .map(|v| v.as_slice())
            .ok_or_else(|| format_err!("linear memory has no data initializers: {:?}", mem_index))
    }
}
