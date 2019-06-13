use crate::bindings::Bindings;
use crate::error::{LucetcError, LucetcErrorKind};
use crate::heap::HeapSettings;
use crate::module::ModuleInfo;
pub use crate::module::{Exportable, TableElems};
use crate::name::Name;
use crate::runtime::{Runtime, RuntimeFunc};
use cranelift_codegen::entity::{EntityRef, PrimaryMap};
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_module::{Backend as ClifBackend, Linkage, Module as ClifModule};
use cranelift_wasm::{
    FuncIndex, Global, GlobalIndex, GlobalInit, MemoryIndex, ModuleEnvironment, SignatureIndex,
    Table, TableIndex,
};
use failure::{format_err, Error, ResultExt};
use lucet_module_data::{
    owned::OwnedLinearMemorySpec, ExportFunction, FunctionIndex as LucetFunctionIndex,
    FunctionMetadata, Global as GlobalVariant, GlobalDef, GlobalSpec, HeapSpec, ImportFunction,
    ModuleData, Signature as LucetSignature, UniqueSignatureIndex,
};
use std::collections::HashMap;
use std::convert::TryFrom;

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
    signature: &'a ir::Signature,
    pub name: Name,
}

impl<'a> RuntimeDecl<'a> {
    pub fn signature(&self) -> &'a ir::Signature {
        self.signature
    }
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
    function_names: PrimaryMap<FuncIndex, Name>,
    imports: Vec<ImportFunction<'a>>,
    exports: Vec<ExportFunction<'a>>,
    table_names: PrimaryMap<TableIndex, (Name, Name)>,
    runtime_names: HashMap<RuntimeFunc, FuncIndex>,
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
        let imports: Vec<ImportFunction<'a>> = Vec::with_capacity(info.imported_funcs.len());
        let table_names = Self::declare_tables(&info, clif_module)?;
        let globals_spec = Self::build_globals_spec(&info)?;
        let linear_memory_spec = Self::build_linear_memory_spec(&info, heap_settings)?;
        let mut decls = Self {
            info,
            function_names: PrimaryMap::new(),
            imports,
            exports: vec![],
            table_names,
            runtime_names: HashMap::new(),
            globals_spec,
            linear_memory_spec,
        };

        Self::declare_funcs(&mut decls, clif_module, bindings)?;
        Self::declare_runtime(&mut decls, clif_module, runtime)?;

        Ok(decls)
    }

    // ********************* Constructor auxillary functions ***********************

    fn declare_funcs<B: ClifBackend>(
        decls: &mut ModuleDecls<'a>,
        clif_module: &mut ClifModule<B>,
        bindings: &Bindings,
    ) -> Result<(), LucetcError> {
        for ix in 0..decls.info.functions.len() {
            let func_index = FuncIndex::new(ix);

            fn export_name_for<'a>(
                func_ix: FuncIndex,
                decls: &mut ModuleDecls<'a>,
            ) -> Option<(String, Linkage)> {
                let export = decls.info.functions.get(func_ix).unwrap();

                if !export.export_names.is_empty() {
                    decls.exports.push(ExportFunction {
                        fn_idx: LucetFunctionIndex::from_u32(decls.function_names.len() as u32),
                        names: export.export_names.clone(),
                    });

                    Some((
                        format!("guest_func_{}", export.export_names[0]),
                        Linkage::Export,
                    ))
                } else {
                    None
                }
            };

            fn import_name_for<'a>(
                func_ix: FuncIndex,
                decls: &mut ModuleDecls<'a>,
                bindings: &Bindings,
            ) -> Result<Option<(String, Linkage)>, failure::Context<LucetcErrorKind>> {
                if let Some((import_mod, import_field)) = decls.info.imported_funcs.get(func_ix) {
                    decls.imports.push(ImportFunction {
                        fn_idx: LucetFunctionIndex::from_u32(decls.function_names.len() as u32),
                        module: import_mod,
                        name: import_field,
                    });
                    let import_symbol = bindings
                        .translate(import_mod, import_field)
                        .context(LucetcErrorKind::TranslatingModule)?;
                    Ok(Some((import_symbol, Linkage::Import)))
                } else {
                    Ok(None)
                }
            };

            let (decl_sym, decl_linkage) = import_name_for(func_index, decls, bindings)?
                .or_else(|| export_name_for(func_index, decls))
                .unwrap_or_else(|| (format!("guest_func_{}", ix), Linkage::Local));

            decls.declare_function(clif_module, decl_sym, decl_linkage, func_index)?;
        }
        Ok(())
    }

    /// Insert a new function into this set of decls and declare it appropriately to `clif_module`.
    /// This is intended for cases where `lucetc` adds a new function that was not present in the
    /// original wasm - in these cases, Cranelift has not already declared the signature or
    /// function type, let alone name, linkage, etc. So we must do that ourselves!
    pub fn declare_new_function<B: ClifBackend>(
        &mut self,
        clif_module: &mut ClifModule<B>,
        decl_sym: String,
        decl_linkage: Linkage,
        signature: ir::Signature,
    ) -> Result<FuncIndex, LucetcError> {
        let (new_funcidx, _) = self.info.declare_func_with_sig(signature);

        self.declare_function(clif_module, decl_sym, decl_linkage, new_funcidx)
    }

    /// The internal side of fixing up a new function declaration. This is also the work that must
    /// be done when building a ModuleDecls record of functions that were described by ModuleInfo.
    fn declare_function<B: ClifBackend>(
        &mut self,
        clif_module: &mut ClifModule<B>,
        decl_sym: String,
        decl_linkage: Linkage,
        func_ix: FuncIndex,
    ) -> Result<FuncIndex, LucetcError> {
        let funcid = clif_module
            .declare_function(
                &decl_sym,
                decl_linkage,
                self.info.signature_for_function(func_ix),
            )
            .context(LucetcErrorKind::TranslatingModule)?;
        self.function_names.push(Name::new_func(decl_sym, funcid));
        Ok(FuncIndex::new(self.function_names.len() - 1))
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
        decls: &mut ModuleDecls<'a>,
        clif_module: &mut ClifModule<B>,
        runtime: Runtime,
    ) -> Result<(), LucetcError> {
        for (func, (symbol, signature)) in runtime.functions.iter() {
            let func_id = decls.declare_new_function(
                clif_module,
                symbol.clone(),
                Linkage::Import,
                signature.clone(),
            )?;

            decls.runtime_names.insert(*func, func_id);
        }
        Ok(())
    }

    fn build_linear_memory_spec(
        info: &ModuleInfo<'a>,
        heap_settings: HeapSettings,
    ) -> Result<Option<OwnedLinearMemorySpec>, LucetcError> {
        use crate::sparsedata::owned_sparse_data_from_initializers;
        if let Some(heap_spec) = Self::build_heap_spec(info, heap_settings)? {
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

    fn build_globals_spec(info: &ModuleInfo<'a>) -> Result<Vec<GlobalSpec<'a>>, LucetcError> {
        let mut globals = Vec::new();
        for ix in 0..info.globals.len() {
            let ix = GlobalIndex::new(ix);
            let g_decl = info.globals.get(ix).unwrap();

            let global = match g_decl.entity.initializer {
                GlobalInit::I32Const(i) => Ok(GlobalVariant::Def(GlobalDef::I32(i))),
                GlobalInit::I64Const(i) => Ok(GlobalVariant::Def(GlobalDef::I64(i))),
                GlobalInit::F32Const(f) => {
                    Ok(GlobalVariant::Def(GlobalDef::F32(f32::from_bits(f))))
                }
                GlobalInit::F64Const(f) => {
                    Ok(GlobalVariant::Def(GlobalDef::F64(f64::from_bits(f))))
                }
                GlobalInit::GetGlobal(ref_ix) => {
                    let ref_decl = info.globals.get(ref_ix).unwrap();
                    if let GlobalInit::Import = ref_decl.entity.initializer {
                        if let Some((module, field)) = info.imported_globals.get(ref_ix) {
                            Ok(GlobalVariant::Import { module, field })
                        } else {
                            Err(format_err!("inconsistent state: global {} is declared as an import but has no entry in imported_globals", ref_ix.as_u32()))
                            .context(LucetcErrorKind::TranslatingModule)
                        }
                    } else {
                        // This WASM restriction may be loosened in the future:
                        Err(format_err!("invalid global declarations: global {} is initialized by referencing another global value, but the referenced global is not an import", ix.as_u32()))
                        .context(LucetcErrorKind::TranslatingModule)
                    }
                }
                GlobalInit::Import => {
                    if let Some((module, field)) = info.imported_globals.get(ix) {
                        Ok(GlobalVariant::Import { module, field })
                    } else {
                        Err(format_err!("inconsistent state: global {} is declared as an import but has no entry in imported_globals", ix.as_u32()))
                        .context(LucetcErrorKind::TranslatingModule)
                    }
                }
            }?;

            globals.push(GlobalSpec::new(global, g_decl.export_names.clone()));
        }
        Ok(globals)
    }

    fn build_heap_spec(
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
        let func_id = *self.runtime_names.get(&runtime_func).unwrap();
        let name = self.function_names.get(func_id).unwrap();
        Ok(RuntimeDecl {
            signature: self.info.signature_for_function(func_id),
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

    pub fn get_module_data(&self) -> Result<ModuleData, LucetcError> {
        let linear_memory = if let Some(ref spec) = self.linear_memory_spec {
            Some(spec.to_ref())
        } else {
            None
        };

        let mut functions: Vec<FunctionMetadata> = Vec::new();
        for fn_index in self.function_names.keys() {
            let decl = self.get_func(fn_index).unwrap();

            // can't use `decl.name` for `FunctionMetadata::name` as `decl` is dropped in the next
            // iteration of this loop.
            let name = self
                .function_names
                .get(fn_index)
                .ok_or_else(|| format_err!("func index out of bounds: {:?}", fn_index))
                .unwrap();

            functions.push(FunctionMetadata {
                signature: decl.signature_index,
                // TODO: this is a best-effort attempt to figure out a useful name.
                // in the future, we should use names from the module names section
                // and maybe use export names as a fallback.
                name: Some(name.symbol()),
            });
        }

        let signatures = self
            .info
            .signatures
            .values()
            .map(|sig| {
                LucetSignature::try_from(sig)
                    .map_err(|e| format_err!("error converting cranelift sig to wasm sig: {:?}", e))
                    .context(LucetcErrorKind::TranslatingModule)
            })
            .collect::<Result<Vec<LucetSignature>, failure::Context<LucetcErrorKind>>>()?;

        Ok(ModuleData::new(
            linear_memory,
            self.globals_spec.clone(),
            functions,
            self.imports.clone(),
            self.exports.clone(),
            signatures,
        ))
    }
}
