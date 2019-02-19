use serde::{Deserialize, Serialize};

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
    pub fn new_def(init_val: i64, export: Option<&'a str>) -> Self {
        Self::new(Global::Def(GlobalDef::new(init_val)), export)
    }
    pub fn new_import(
        import_module: &'a str,
        import_field: &'a str,
        export: Option<&'a str>,
    ) -> Self {
        Self::new(
            Global::Import(GlobalImport::new(import_module, import_field)),
            export,
        )
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
    init_val: i64,
}
impl GlobalDef {
    pub fn new(init_val: i64) -> Self {
        Self { init_val }
    }

    pub fn init_val(&self) -> i64 {
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

/////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct OwnedGlobalSpec {
    global: OwnedGlobal,
    export: Option<String>,
}

impl OwnedGlobalSpec {
    pub fn new(global: OwnedGlobal, export: Option<String>) -> Self {
        Self { global, export }
    }

    pub fn new_def(init_val: i64, export: Option<String>) -> Self {
        Self::new(OwnedGlobal::Def(GlobalDef::new(init_val)), export)
    }

    pub fn new_import(import_module: String, import_field: String, export: Option<String>) -> Self {
        Self::new(
            OwnedGlobal::Import(OwnedGlobalImport::new(import_module, import_field)),
            export,
        )
    }

    pub fn get_ref(&self) -> GlobalSpec {
        let export = match &self.export {
            Some(e) => Some(e.as_str()),
            None => None,
        };
        GlobalSpec::new(self.global.get_ref(), export)
    }
}

pub enum OwnedGlobal {
    Def(GlobalDef),
    Import(OwnedGlobalImport),
}

impl OwnedGlobal {
    pub fn get_ref(&self) -> Global {
        match self {
            OwnedGlobal::Def(d) => Global::Def(d.clone()),
            OwnedGlobal::Import(i) => Global::Import(i.get_ref()),
        }
    }
}

pub struct OwnedGlobalImport {
    module: String,
    field: String,
}

impl OwnedGlobalImport {
    pub fn new(module: String, field: String) -> Self {
        Self { module, field }
    }
    pub fn get_ref(&self) -> GlobalImport {
        GlobalImport::new(&self.module, &self.field)
    }
}
