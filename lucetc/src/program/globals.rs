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
}

impl GlobalImport {
    pub fn new(importentry: &ImportEntry, global_type: GlobalType) -> Self {
        Self {
            module: String::from(importentry.module()),
            field: String::from(importentry.field()),
            global_type: global_type,
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
    pub fn export(&self) -> Option<String> {
        self.export.clone()
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
}
