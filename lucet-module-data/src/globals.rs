use serde::{Deserialize, Serialize};

/// A WebAssembly global along with its export specification.
///
/// The lifetime parameter exists to support zero-copy deserialization for the `&str` fields at the
/// leaves of the structure. For a variant with owned types at the leaves, see
/// [`OwnedGlobalSpec`](owned/struct.OwnedGlobalSpec.html).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalSpec<'a> {
    #[serde(borrow)]
    global: Global<'a>,
    export_names: Vec<&'a str>,
}

impl<'a> GlobalSpec<'a> {
    pub fn new(global: Global<'a>, export_names: Vec<&'a str>) -> Self {
        Self { global, export_names }
    }

    /// Create a new global definition with an initial value and export names.
    pub fn new_def(init_val: i64, export_names: Vec<&'a str>) -> Self {
        Self::new(
            Global::Def {
                def: GlobalDef::new(init_val),
            },
            export_names,
        )
    }

    /// Create a new global import definition with a module and field name, and export names.
    pub fn new_import(module: &'a str, field: &'a str, export_names: Vec<&'a str>) -> Self {
        Self::new(Global::Import { module, field }, export_names)
    }

    pub fn global(&self) -> &Global {
        &self.global
    }

    pub fn export_names(&self) -> &[&str] {
        &self.export_names
    }

    pub fn is_internal(&self) -> bool {
        self.export_names.len() == 0
    }
}

/// A WebAssembly global is either defined locally, or is defined in relation to a field of another
/// WebAssembly module.
///
/// Lucet currently does not support import globals, but we support the metadata for future
/// compatibility.
///
/// The lifetime parameter exists to support zero-copy deserialization for the `&str` fields at the
/// leaves of the structure. For a variant with owned types at the leaves, see
/// [`OwnedGlobal`](owned/struct.OwnedGlobal.html).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Global<'a> {
    Def { def: GlobalDef },
    Import { module: &'a str, field: &'a str },
}

/// A global definition.
///
/// Currently we cast everything to an `i64`, but in the future this may have explicit variants for
/// the different WebAssembly scalar types.
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

/////////////////////////////////////////////////////////////////////////////////////////////////////////

/// A variant of [`GlobalSpec`](../struct.GlobalSpec.html) with owned strings throughout.
///
/// This type is useful when directly building up a value to be serialized.
pub struct OwnedGlobalSpec {
    global: OwnedGlobal,
    export_names: Vec<String>,
}

impl OwnedGlobalSpec {
    pub fn new(global: OwnedGlobal, export_names: Vec<String>) -> Self {
        Self { global, export_names }
    }

    /// Create a new global definition with an initial value and export names.
    pub fn new_def(init_val: i64, export_names: Vec<String>) -> Self {
        Self::new(
            OwnedGlobal::Def {
                def: GlobalDef::new(init_val),
            },
            export_names,
        )
    }

    /// Create a new global import definition with a module and field name, and export names.
    pub fn new_import(module: String, field: String, export_names: Vec<String>) -> Self {
        Self::new(OwnedGlobal::Import { module, field }, export_names)
    }

    /// Create a [`GlobalSpec`](../struct.GlobalSpec.html) backed by the values in this
    /// `OwnedGlobalSpec`.
    pub fn to_ref<'a>(&'a self) -> GlobalSpec<'a> {
        GlobalSpec::new(self.global.to_ref(), self.export_names.iter().map(|x| x.as_str()).collect())
    }
}

/// A variant of [`Global`](../struct.Global.html) with owned strings throughout.
///
/// This type is useful when directly building up a value to be serialized.
pub enum OwnedGlobal {
    Def { def: GlobalDef },
    Import { module: String, field: String },
}

impl OwnedGlobal {
    /// Create a [`Global`](../struct.Global.html) backed by the values in this `OwnedGlobal`.
    pub fn to_ref<'a>(&'a self) -> Global<'a> {
        match self {
            OwnedGlobal::Def { def } => Global::Def { def: def.clone() },
            OwnedGlobal::Import { module, field } => Global::Import {
                module: module.as_str(),
                field: field.as_str(),
            },
        }
    }
}
