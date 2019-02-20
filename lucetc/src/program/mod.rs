pub mod data;
pub mod function;
pub mod globals;
pub mod init_expr;
pub mod memory;
pub mod names;
pub mod runtime;
pub mod table;
pub mod types;

pub use self::data::{module_data, DataInit};
pub use self::function::{Function, FunctionDef, FunctionImport, FunctionRuntime};
pub use self::globals::{Global, GlobalDef, GlobalImport};
pub use self::memory::{create_heap_spec, empty_heap_spec, HeapSettings, HeapSpec, MemorySpec};
pub use self::names::{module_names, ModuleNames};
pub use self::runtime::Runtime;
pub use self::table::{TableBuilder, TableDef};
pub use self::types::{CtonSignature, FunctionSig};

use crate::bindings::Bindings;
use crate::error::{LucetcError, LucetcErrorKind};
use crate::program::init_expr::const_init_expr;
use failure::{format_err, ResultExt};
use parity_wasm::elements::{External, FuncBody, MemoryType, Module, TableElementType, Type};
use pwasm_validation::validate_module;
use std::collections::{hash_map::Entry, HashMap};

pub struct Program {
    module: Module,

    globals: Vec<Global>,
    tables: Vec<TableDef>,
    runtime: Runtime,
    heap_settings: HeapSettings,

    defined_funcs: Vec<FunctionDef>,
    defined_memory: Option<MemorySpec>,

    import_functions: Vec<FunctionImport>,
    import_memory: Option<MemorySpec>,
}

impl Program {
    pub fn new(
        module: Module,
        bindings: Bindings,
        heap_settings: HeapSettings,
    ) -> Result<Self, LucetcError> {
        let module = module.parse_names().map_err(|(es, _)| {
            format_err!("could not parse some of the name sections: {:?}", es)
        })?;

        let module = validate_module(module)?.unwrap();
        let names = module_names(&module)?;
        let imports = module_imports(&module, bindings, &names)?;
        let defs = module_definitions(&module, &imports, &names)?;
        let tables = module_tables(&module, imports.tables)?;
        let globals = module_globals(imports.globals, defs.globals);
        let runtime = Runtime::liblucet_runtime_c();
        Ok(Self {
            module,
            globals,
            tables,
            runtime,
            heap_settings,
            defined_funcs: defs.funcs,
            defined_memory: defs.memory,

            import_functions: imports.functions,
            import_memory: imports.memory,
        })
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn get_function(&self, index: u32) -> Result<&Function, LucetcError> {
        if let Some(fnindex) = index.checked_sub(self.import_functions.len() as u32) {
            if let Some(def) = self.defined_funcs.get(fnindex as usize) {
                Ok(def)
            } else {
                Err(format_err!("function index {} is out of bounds", index))?
            }
        } else {
            Ok(self
                .import_functions
                .get(index as usize)
                .expect("function import index was checked"))
        }
    }

    pub fn globals(&self) -> &[Global] {
        self.globals.as_ref()
    }

    pub fn get_signature(&self, index: u32) -> Result<FunctionSig, LucetcError> {
        module_get_signature(&self.module, index)
    }

    pub fn tables(&self) -> &[TableDef] {
        &self.tables
    }

    pub fn get_table(&self, index: u32) -> Result<&TableDef, LucetcError> {
        let tbl = self
            .tables
            .get(index as usize)
            .ok_or_else(|| format_err!("table out of range: {}", index))?;
        Ok(tbl)
    }

    pub fn runtime_functions(&self) -> &[FunctionRuntime] {
        self.runtime.functions()
    }

    pub fn get_runtime_function(&self, name: &str) -> Result<&FunctionRuntime, LucetcError> {
        self.runtime.get_symbol(name)
    }

    pub fn heap_spec(&self) -> HeapSpec {
        if let Some(ref mem_spec) = self.import_memory {
            create_heap_spec(mem_spec, &self.heap_settings)
        } else if let Some(ref mem_spec) = self.defined_memory {
            create_heap_spec(mem_spec, &self.heap_settings)
        } else {
            empty_heap_spec()
        }
    }

    pub fn data_initializers(&self) -> Result<Vec<DataInit>, LucetcError> {
        let v = module_data(&self.module)?;
        Ok(v)
    }

    pub fn function_body(&self, def: &FunctionDef) -> &FuncBody {
        let bodies = self
            .module
            .code_section()
            .map(|s| s.bodies())
            .unwrap_or(&[]);
        let fn_index_base = self.import_functions.len();
        &bodies
            .get(def.wasmidx as usize - fn_index_base)
            .expect("functiondef points to valid body")
    }

    pub fn defined_functions(&self) -> &[FunctionDef] {
        self.defined_funcs.as_ref()
    }

    pub fn import_functions(&self) -> &[FunctionImport] {
        self.import_functions.as_ref()
    }
}

pub struct ModuleTables {
    pub tables: Vec<TableDef>,
}

enum TableDecl {
    Import(TableDef),
    Def(TableBuilder),
}

fn memory_spec(mem: &MemoryType) -> MemorySpec {
    let limits = mem.limits();
    MemorySpec {
        initial_pages: limits.initial(),
        max_pages: limits.maximum(),
    }
}

fn module_get_signature(module: &Module, index: u32) -> Result<FunctionSig, LucetcError> {
    let type_entry = module
        .type_section()
        .ok_or_else(|| LucetcErrorKind::Other("no types in this module".to_owned()))?
        .types()
        .get(index as usize)
        .ok_or_else(|| LucetcErrorKind::Other(format!("no signature for {}", index)))?;
    match type_entry {
        &Type::Function(ref ftype) => Ok(FunctionSig::new(index, ftype)),
    }
}

struct ModuleImports {
    memory: Option<MemorySpec>,
    functions: Vec<FunctionImport>,
    globals: Vec<GlobalImport>,
    tables: Vec<TableDef>,
}

impl ModuleImports {
    pub fn function_index_base(&self) -> u32 {
        self.functions.len() as u32
    }
    pub fn global_index_base(&self) -> u32 {
        self.globals.len() as u32
    }
}

fn module_imports(
    module: &Module,
    bindings: Bindings,
    names: &ModuleNames,
) -> Result<ModuleImports, LucetcError> {
    let mut memory = None;
    let mut functions = Vec::new();
    let mut globals = Vec::new();
    let mut tables = Vec::new();
    if let Some(import_section) = module.import_section() {
        for entry in import_section.entries().iter() {
            match entry.external() {
                &External::Function(typeix) => {
                    let functionix = functions.len() as u32;
                    let ftype = module_get_signature(&module, typeix)?;
                    functions.push(FunctionImport::new(functionix, entry, ftype, &bindings)?)
                }
                &External::Global(ref gty) => {
                    let globalix = globals.len() as u32;
                    globals.push(GlobalImport::new(
                        entry,
                        gty.clone(),
                        names.global_symbol(globalix),
                    ))
                }

                &External::Table(ref tty) => {
                    let tableix = tables.len() as u32;
                    let builder =
                        TableBuilder::new(tableix, tty.limits().initial(), tty.limits().maximum())?;
                    tables.push(builder.finalize());
                }
                &External::Memory(ref mem) => memory = Some(memory_spec(mem)),
            }
        }
    }
    Ok(ModuleImports {
        memory,
        functions,
        globals,
        tables,
    })
}

struct ModuleDefs {
    funcs: Vec<FunctionDef>,
    globals: Vec<GlobalDef>,
    memory: Option<MemorySpec>,
}

fn module_definitions(
    module: &Module,
    imports: &ModuleImports,
    names: &ModuleNames,
) -> Result<ModuleDefs, LucetcError> {
    let mut memory = None;

    let decls = module
        .function_section()
        .map(|s| s.entries())
        .unwrap_or(&[]);
    let mut funcs = Vec::new();
    for (ix, decl) in decls.iter().enumerate() {
        let funcindex = ix as u32 + imports.function_index_base();

        funcs.push(FunctionDef::new(
            funcindex,
            module_get_signature(&module, decl.type_ref()).expect("signature for func must exist"),
            names.function_exported(funcindex),
            names.function_symbol(funcindex),
        ))
    }

    if let Some(memory_types) = module.memory_section().map(|s| s.entries()) {
        if memory_types.len() > 1 {
            Err(format_err!("multiple memories are not supported"))?
        }
        for memory_type in memory_types.iter() {
            memory = Some(memory_spec(memory_type))
        }
    }

    let mut globals = Vec::new();
    if let Some(global_decls) = module.global_section().map(|s| s.entries()) {
        for (ix, decl) in global_decls.iter().enumerate() {
            let globalindex = ix as u32 + imports.global_index_base();
            globals.push(GlobalDef::new(
                decl.global_type().clone(),
                decl.init_expr(),
                names.global_symbol(globalindex),
            )?)
        }
    }
    Ok(ModuleDefs {
        funcs,
        globals,
        memory,
    })
}

fn module_tables(
    module: &Module,
    import_tables: Vec<TableDef>,
) -> Result<Vec<TableDef>, LucetcError> {
    let mut tables = HashMap::new();
    for tbl in import_tables.iter() {
        tables.insert(tbl.index(), TableDecl::Import(tbl.clone()));
    }
    if let Some(table_section) = module.table_section() {
        for table_type in table_section.entries().iter() {
            match table_type.elem_type() {
                TableElementType::AnyFunc => {
                    let section_index = 0;
                    match tables.entry(section_index) {
                        Entry::Occupied(occ) => match occ.get() {
                            TableDecl::Import(_) => Err(format_err!(
                                "Cannot define table {}: already declared as import",
                                section_index
                            ))?,
                            TableDecl::Def(_) => Err(format_err!(
                                "Cannot define table {}: already defined",
                                section_index
                            ))?,
                        },
                        Entry::Vacant(vac) => {
                            vac.insert(TableDecl::Def(TableBuilder::new(
                                section_index,
                                table_type.limits().initial(),
                                table_type.limits().maximum(),
                            )?));
                        }
                    }
                }
            }
        }
        if let Some(element_section) = module.elements_section() {
            for (segment_ix, element_segment) in element_section.entries().iter().enumerate() {
                let table_ix = element_segment.index();
                let offs: i64 = const_init_expr(
                    element_segment
                        .offset()
                        .as_ref()
                        .ok_or(format_err!("Offset not found"))?
                        .code(),
                )
                .context(LucetcErrorKind::Other(format!(
                    "in element segment offset for table {}, segment {}",
                    table_ix, segment_ix
                )))?;
                // Ensure its safe to make into an i32:
                assert!(offs >= <i32>::min_value() as i64 && offs <= <i32>::max_value() as i64);
                match tables.get_mut(&table_ix) {
                    Some(TableDecl::Def(ref mut builder)) => builder
                        .push_elements(offs as i32, element_segment.members().to_vec())
                        .context(LucetcErrorKind::Other(format!(
                            "in elements for table {}, segment {}",
                            table_ix, segment_ix
                        )))?,
                    Some(TableDecl::Import(_)) => Err(format_err!(
                        "Cannot define element for imported table {}",
                        table_ix
                    ))?,
                    None => Err(format_err!(
                        "Cannot define element for undeclared table {}",
                        table_ix
                    ))?,
                }
            }
        }
    }
    let tables = tables
        .values()
        .map(|decl| match decl {
            TableDecl::Import(def) => def.clone(),
            TableDecl::Def(builder) => builder.finalize(),
        })
        .collect();
    Ok(tables)
}

fn module_globals(imports: Vec<GlobalImport>, defs: Vec<GlobalDef>) -> Vec<Global> {
    let mut globals = Vec::new();
    for g in imports {
        globals.push(Global::Import(g.clone()));
    }
    for g in defs {
        globals.push(Global::Def(g.clone()));
    }
    globals
}
