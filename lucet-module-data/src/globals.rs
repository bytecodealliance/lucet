use crate::lucet_module_data_capnp::{global_def, global_import, global_spec};
use failure::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSpec<'a> {
    global: Global<'a>,
    export: Option<&'a str>,
}

impl<'a> GlobalSpec<'a> {
    pub fn new(global: Global<'a>, export: Option<&'a str>) -> Self {
        Self { global, export }
    }
    pub fn global(&self) -> &Global {
        &self.global
    }
    pub fn export(&self) -> Option<&str> {
        self.export
    }

    pub(crate) fn read(reader: global_spec::Reader<'a>) -> Result<Self, Error> {
        let global = Global::read(reader.get_global())?;
        let export = match reader
            .get_export()
            .which()
            .map_err(|e| format_err!("in global_spec field export: {}", e))?
        {
            global_spec::export::Which::Name(n) => {
                Some(n.map_err(|e| format_err!("export missing name: {}", e))?)
            }
            global_spec::export::Which::None(_) => None,
        };
        Ok(Self { global, export })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Global<'a> {
    Def(GlobalDef),
    Import(GlobalImport<'a>),
}

impl<'a> Global<'a> {
    pub(crate) fn read(reader: global_spec::global::Reader<'a>) -> Result<Self, Error> {
        match reader
            .which()
            .map_err(|e| format_err!("in global_spec field global: {}", e))?
        {
            global_spec::global::Which::Def(def) => {
                let reader =
                    def.map_err(|e| format_err!("in global_spec.global variant def: {}", e))?;
                Ok(Global::Def(GlobalDef::read(reader)?))
            }
            global_spec::global::Which::Import(imp) => {
                let reader =
                    imp.map_err(|e| format_err!("in global_spec.global variant import: {}", e))?;
                Ok(Global::Import(GlobalImport::read(reader)?))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalDef {
    init_val: u64,
}
impl GlobalDef {
    pub fn new(init_val: u64) -> Self {
        Self { init_val }
    }

    pub fn init_val(&self) -> u64 {
        self.init_val
    }

    pub(crate) fn read<'a>(reader: global_def::Reader<'a>) -> Result<Self, Error> {
        let init_val = reader.get_init_val();
        Ok(Self::new(init_val))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalImport<'a> {
    module: &'a str,
    field: &'a str,
}

impl<'a> GlobalImport<'a> {
    pub fn new(module: &'a str, field: &'a str) -> Self {
        Self { module, field }
    }

    pub fn module(&self) -> &str {
        self.module
    }

    pub fn field(&self) -> &str {
        self.field
    }

    pub(crate) fn read(reader: global_import::Reader<'a>) -> Result<Self, Error> {
        let module = reader
            .get_module()
            .map_err(|e| format_err!("in global_import field module: {}", e))?;
        let field = reader
            .get_field()
            .map_err(|e| format_err!("in global_import field field: {}", e))?;
        Ok(Self::new(module, field))
    }
}
