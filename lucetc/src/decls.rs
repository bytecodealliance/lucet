use crate::bindings::Bindings;
use crate::error::{LucetcError, LucetcErrorKind};
use crate::heap::HeapSettings;
use crate::module::ModuleInfo;
pub use crate::module::{Exportable, TableElems};
use crate::name::Name;
use crate::runtime::{Runtime, RuntimeFunc};
use cranelift_codegen::entity::{entity_impl, EntityRef, PrimaryMap};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_module::{Backend as ClifBackend, Linkage, Module as ClifModule};
use cranelift_wasm::{
    FuncIndex, Global, GlobalIndex, GlobalInit, MemoryIndex, ModuleEnvironment, SignatureIndex,
    Table, TableIndex,
};
use failure::{format_err, Error, ResultExt};
use lucet_module_data::{
    owned::OwnedLinearMemorySpec, Global as GlobalVariant, GlobalDef, GlobalSpec, HeapSpec,
    ModuleData,
};
use std::collections::HashMap;

/// UniqueSignatureIndex names a signature after collapsing duplicate signatures to a single
/// identifier, whereas SignatureIndex is directly what the original module specifies, and may
/// specify duplicates of types that are structurally equal.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct UniqueSignatureIndex(u32);
entity_impl!(UniqueSignatureIndex);

#[derive(Debug)]
pub struct FunctionDecl<'a> {
    pub import_name: Option<(&'a str, &'a str)>,
    pub export_names: Vec<&'a str>,
    pub signature_index: UniqueSignatureIndex,
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

#[derive(Debug)]
/// Function provided by lucet-runtime to be called from generated code, e.g. memory size & grow
/// functions.
pub struct RuntimeDecl<'a> {
    pub signature: &'a ir::Signature,
    pub name: Name,
}

#[derive(Debug)]
pub struct TableDecl<'a> {
    pub import_name: Option<(&'a str, &'a str)>,
    pub export_names: Vec<&'a str>,
    pub table: &'a Table,
    pub elems: &'a [TableElems],
    pub contents_name: Name,
    pub len_name: Name,
}

pub struct ModuleDecls<'a> {
    info: ModuleInfo<'a>,
    runtime: Runtime,
    function_names: PrimaryMap<FuncIndex, Name>,
    table_names: PrimaryMap<TableIndex, (Name, Name)>,
    runtime_names: HashMap<RuntimeFunc, Name>,
    globals_spec: Vec<GlobalSpec<'a>>,
    linear_memory_spec: Option<OwnedLinearMemorySpec>,
}

impl<'a> ModuleDecls<'a> {
    pub fn new<B: ClifBackend>(
        info: ModuleInfo<'a>,
        clif_module: &mut ClifModule<B>,
        bindings: &Bindings,
        runtime: Runtime,
        heap_settings: HeapSettings,
    ) -> Result<Self, LucetcError> {
        let function_names = Self::declare_funcs(&info, clif_module, bindings)?;
        let table_names = Self::declare_tables(&info, clif_module)?;
        let runtime_names = Self::declare_runtime(&runtime, clif_module)?;
        let globals_spec = Self::declare_globals_spec(&info)?;
        let linear_memory_spec = Self::declare_linear_memory_spec(&info, heap_settings)?;
        Ok(Self {
            info,
            function_names,
            table_names,
            runtime_names,
            runtime,
            globals_spec,
            linear_memory_spec,
        })
    }

    // ********************* Constructor auxillary functions ***********************

    fn declare_funcs<B: ClifBackend>(
        info: &ModuleInfo<'a>,
        clif_module: &mut ClifModule<B>,
        bindings: &Bindings,
    ) -> Result<PrimaryMap<FuncIndex, Name>, LucetcError> {
        let mut function_names = PrimaryMap::new();
        for ix in 0..info.functions.len() {
            let func_index = FuncIndex::new(ix);
            let exportable_sigix = info.functions.get(func_index).unwrap();
            let inner_sig_index = info.signature_mapping.get(exportable_sigix.entity).unwrap();
            let signature = info.signatures.get(*inner_sig_index).unwrap();
            let name = if let Some((import_mod, import_field)) = info.imported_funcs.get(func_index)
            {
                let import_symbol = bindings
                    .translate(import_mod, import_field)
                    .context(LucetcErrorKind::TranslatingModule)?;
                let funcid = clif_module
                    .declare_function(&import_symbol, Linkage::Import, signature)
                    .context(LucetcErrorKind::TranslatingModule)?;
                Name::new_func(import_symbol, funcid)
            } else {
                if exportable_sigix.export_names.is_empty() {
                    let def_symbol = format!("guest_func_{}", ix);
                    let funcid = clif_module
                        .declare_function(&def_symbol, Linkage::Local, signature)
                        .context(LucetcErrorKind::TranslatingModule)?;
                    Name::new_func(def_symbol, funcid)
                } else {
                    let export_symbol = format!("guest_func_{}", exportable_sigix.export_names[0]);
                    let funcid = clif_module
                        .declare_function(&export_symbol, Linkage::Export, signature)
                        .context(LucetcErrorKind::TranslatingModule)?;
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
                .declare_data(&def_symbol, Linkage::Export, false)
                .context(LucetcErrorKind::TranslatingModule)?;
            let def_name = Name::new_data(def_symbol, def_data_id);

            let len_symbol = format!("guest_table_{}_len", ix);
            let len_data_id = clif_module
                .declare_data(&len_symbol, Linkage::Export, false)
                .context(LucetcErrorKind::TranslatingModule)?;
            let len_name = Name::new_data(len_symbol, len_data_id);

            table_names.push((def_name, len_name));
        }
        Ok(table_names)
    }

    fn declare_runtime<B: ClifBackend>(
        runtime: &Runtime,
        clif_module: &mut ClifModule<B>,
    ) -> Result<HashMap<RuntimeFunc, Name>, LucetcError> {
        let mut runtime_names: HashMap<RuntimeFunc, Name> = HashMap::new();
        for (func, (symbol, signature)) in runtime.functions.iter() {
            let funcid = clif_module
                .declare_function(&symbol, Linkage::Import, signature)
                .context(LucetcErrorKind::TranslatingModule)?;
            let name = Name::new_func(symbol.clone(), funcid);

            runtime_names.insert(*func, name);
        }
        Ok(runtime_names)
    }

    fn declare_linear_memory_spec(
        info: &ModuleInfo<'a>,
        heap_settings: HeapSettings,
    ) -> Result<Option<OwnedLinearMemorySpec>, LucetcError> {
        use crate::sparsedata::owned_sparse_data_from_initializers;
        if let Some(heap_spec) = Self::declare_heap_spec(info, heap_settings)? {
            let data_initializers = info
                .data_initializers
                .get(&MemoryIndex::new(0))
                .expect("heap spec implies data initializers should exist");
            let sparse_data = owned_sparse_data_from_initializers(data_initializers, &heap_spec)?;

            Ok(Some(OwnedLinearMemorySpec {
                heap: heap_spec,
                initializer: sparse_data,
            }))
        } else {
            Ok(None)
        }
    }

    fn declare_globals_spec(info: &ModuleInfo<'a>) -> Result<Vec<GlobalSpec<'a>>, LucetcError> {
        let mut globals = Vec::new();
        for ix in 0..info.globals.len() {
            let ix = GlobalIndex::new(ix);
            let g_decl = info.globals.get(ix).unwrap();
            let g_import = info.imported_globals.get(ix);
            let g_variant = if let Some((module, field)) = g_import {
                GlobalVariant::Import { module, field }
            } else {
                let init_val = match g_decl.entity.initializer {
                    // Need to fix global spec in ModuleData and the runtime to support more:
                    GlobalInit::I32Const(i) => i as i64,
                    GlobalInit::I64Const(i) => i,
                    _ => Err(format_err!(
                        "non-integer global initializer: {:?}",
                        g_decl.entity
                    ))
                    .context(LucetcErrorKind::Unsupported)?,
                };
                GlobalVariant::Def {
                    def: GlobalDef::new(init_val),
                }
            };
            globals.push(GlobalSpec::new(g_variant, None));
        }
        Ok(globals)
    }

    fn declare_heap_spec(
        info: &ModuleInfo<'a>,
        heap_settings: HeapSettings,
    ) -> Result<Option<HeapSpec>, LucetcError> {
        match info.memories.len() {
            0 => Ok(None),
            1 => {
                let memory = info
                    .memories
                    .get(MemoryIndex::new(0))
                    .expect("memory in range")
                    .entity;

                let wasm_page: u64 = 64 * 1024;
                let initial_size = memory.minimum as u64 * wasm_page;

                let reserved_size = std::cmp::max(initial_size, heap_settings.min_reserved_size);
                if reserved_size > heap_settings.max_reserved_size {
                    Err(format_err!(
                        "module reserved size ({}) exceeds max reserved size ({})",
                        reserved_size,
                        heap_settings.max_reserved_size
                    ))
                    .context(LucetcErrorKind::MemorySpecs)?;
                }
                // Find the max size permitted by the heap and the memory spec
                let max_size = memory.maximum.map(|pages| pages as u64 * wasm_page);
                Ok(Some(HeapSpec {
                    reserved_size,
                    guard_size: heap_settings.guard_size,
                    initial_size: initial_size,
                    max_size: max_size,
                }))
            }
            _ => Err(format_err!("lucetc only supports memory 0"))
                .context(LucetcErrorKind::Unsupported)?,
        }
    }
    // ********************* Public Interface **************************

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
        let signature_index = self.get_signature_uid(exportable_sigix.entity).unwrap();
        let signature = self.info.signatures.get(signature_index).unwrap();
        let import_name = self.info.imported_funcs.get(func_index);
        Ok(FunctionDecl {
            signature,
            signature_index,
            export_names: exportable_sigix.export_names.clone(),
            import_name: import_name.cloned(),
            name: name.clone(),
        })
    }

    pub fn get_start_func(&self) -> Option<FuncIndex> {
        self.info.start_func.clone()
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
        let elems = self.info.table_elems.get(&table_index).unwrap().as_slice();
        Ok(TableDecl {
            table: &exportable_tbl.entity,
            elems,
            export_names: exportable_tbl.export_names.clone(),
            import_name: import_name.cloned(),
            contents_name: contents_name.clone(),
            len_name: len_name.clone(),
        })
    }

    pub fn get_signature(&self, signature_index: SignatureIndex) -> Result<&ir::Signature, Error> {
        self.get_signature_uid(signature_index).and_then(|uid| {
            self.info
                .signatures
                .get(uid)
                .ok_or_else(|| format_err!("signature out of bounds: {:?}", uid))
        })
    }

    pub fn get_signature_uid(
        &self,
        signature_index: SignatureIndex,
    ) -> Result<UniqueSignatureIndex, Error> {
        self.info
            .signature_mapping
            .get(signature_index)
            .map(|x| *x)
            .ok_or_else(|| format_err!("signature out of bounds: {:?}", signature_index))
    }

    pub fn get_global(&self, global_index: GlobalIndex) -> Result<&Exportable<Global>, Error> {
        self.info
            .globals
            .get(global_index)
            .ok_or_else(|| format_err!("global out of bounds: {:?}", global_index))
    }

    pub fn get_heap(&self) -> Option<&HeapSpec> {
        if let Some(ref spec) = self.linear_memory_spec {
            Some(&spec.heap)
        } else {
            None
        }
    }

    pub fn get_module_data(&self) -> ModuleData {
        let linear_memory = if let Some(ref spec) = self.linear_memory_spec {
            Some(spec.to_ref())
        } else {
            None
        };
        ModuleData::new(linear_memory, self.globals_spec.clone())
    }
}
