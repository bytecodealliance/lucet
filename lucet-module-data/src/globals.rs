use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalSpec<'a> {
    #[serde(borrow)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Global<'a> {
    Def(GlobalDef),
    #[serde(borrow)]
    Import(GlobalImport<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}
