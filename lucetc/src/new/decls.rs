use crate::compiler::name::Name;
use crate::error::LucetcError;
use crate::new::module::ModuleInfo;
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_module::{Backend as ClifBackend, Module as ClifModule};
use cranelift_wasm::{
    FuncIndex, Global, GlobalIndex, Memory, MemoryIndex, ModuleEnvironment, Table, TableIndex,
};

pub struct ModuleDecls<'a> {
    info: ModuleInfo<'a>,
}

pub struct Entity<'a, T> {
    pub import_name: Option<(&'a str, &'a str)>,
    pub export_names: Vec<&'a str>,
    pub entity: &'a T,
    pub name: Name,
}

impl<'a, T> Entity<'a, T> {
    pub fn defined(&self) -> bool {
        self.import_name.is_none()
    }
    pub fn imported(&self) -> bool {
        !self.defined()
    }
    pub fn exported(&self) -> bool {
        !self.export_names.is_empty()
    }
}

impl<'a> ModuleDecls<'a> {
    pub fn declare<B: ClifBackend>(
        info: ModuleInfo<'a>,
        _clif_module: &mut ClifModule<B>,
    ) -> Result<Self, LucetcError> {
        Ok(Self { info })
    }

    pub fn target_config(&self) -> TargetFrontendConfig {
        self.info.target_config()
    }

    pub fn function_bodies(
        &self,
    ) -> impl Iterator<Item = (Entity<ir::Signature>, &(&'a [u8], usize))> {
        Box::new(
            self.info
                .function_bodies
                .iter()
                .map(move |(fidx, code)| (self.get_func(*fidx).unwrap(), code)),
        )
    }

    pub fn get_func(&self, func_index: FuncIndex) -> Option<Entity<ir::Signature>> {
        unimplemented!()
    }

    pub fn get_global(&self, global_index: GlobalIndex) -> Option<Entity<Global>> {
        unimplemented!()
    }

    pub fn get_table(&self, table_index: TableIndex) -> Option<Entity<Table>> {
        unimplemented!()
    }

    pub fn get_memory(&self, memory_index: MemoryIndex) -> Option<Entity<Memory>> {
        unimplemented!()
    }
}
