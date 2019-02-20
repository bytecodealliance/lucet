use super::init_expr::const_init_expr;
use super::types::cton_valuetype;
use crate::error::LucetcError;
use cranelift_codegen::ir;
use parity_wasm::elements::{GlobalType, ImportEntry, InitExpr};

#[derive(Debug, Clone)]
pub struct GlobalImport {
    module: String,
    field: String,
    pub global_type: GlobalType,
    export: Option<String>,
}

impl GlobalImport {
    pub fn new(importentry: &ImportEntry, global_type: GlobalType, export: Option<String>) -> Self {
        Self {
            module: String::from(importentry.module()),
            field: String::from(importentry.field()),
            global_type,
            export,
        }
    }

    pub fn cton_type(&self) -> ir::Type {
        cton_valuetype(&self.global_type.content_type())
    }

    pub fn module(&self) -> &str {
        self.module.as_str()
    }

    pub fn field(&self) -> &str {
        self.field.as_str()
    }

    pub fn export(&self) -> Option<&str> {
        match self.export {
            Some(ref ex) => Some(ex.as_str()),
            None => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalDef {
    global_type: GlobalType,
    value: i64,
    export: Option<String>,
}

impl GlobalDef {
    pub fn new(
        global_type: GlobalType,
        init_expr: &InitExpr,
        export: Option<String>,
    ) -> Result<Self, LucetcError> {
        let value = const_init_expr(init_expr.code())?;
        Ok(Self {
            global_type: global_type,
            value: value,
            export: export,
        })
    }
    pub fn cton_type(&self) -> ir::Type {
        cton_valuetype(&self.global_type.content_type())
    }
    pub fn value(&self) -> i64 {
        self.value
    }
    pub fn export(&self) -> Option<&str> {
        match self.export {
            Some(ref ex) => Some(ex.as_str()),
            None => None,
        }
    }
}

pub enum Global {
    Import(GlobalImport),
    Def(GlobalDef),
}

impl Global {
    pub fn cton_type(&self) -> ir::Type {
        match self {
            &Global::Import(ref globalimport) => globalimport.cton_type(),
            &Global::Def(ref globaldef) => globaldef.cton_type(),
        }
    }

    pub fn as_import(&self) -> Option<&GlobalImport> {
        match self {
            Global::Import(i) => Some(i),
            Global::Def(_) => None,
        }
    }

    pub fn as_def(&self) -> Option<&GlobalDef> {
        match self {
            Global::Import(_) => None,
            Global::Def(d) => Some(d),
        }
    }

    pub fn export(&self) -> Option<&str> {
        match self {
            Global::Import(i) => i.export(),
            Global::Def(d) => d.export(),
        }
    }
}

use lucet_module_data as data;

impl Global {
    pub fn to_spec(&self) -> data::GlobalSpec {
        let global = match self {
            Global::Import(i) => data::Global::Import {
                module: i.module(),
                field: i.field(),
            },
            Global::Def(d) => data::Global::Def {
                def: data::GlobalDef::new(d.value()),
            },
        };
        let export = self.export();
        data::GlobalSpec::new(global, export)
    }
}
