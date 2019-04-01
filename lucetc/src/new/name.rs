use cranelift_codegen::ir::ExternalName;
use cranelift_module::{DataId, FuncId, FuncOrDataId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name {
    symbol: String,
    id: FuncOrDataId,
}

impl Name {
    pub fn new(symbol: String, id: FuncOrDataId) -> Self {
        Self {
            symbol: symbol,
            id: id,
        }
    }

    pub fn new_func(symbol: String, id: FuncId) -> Self {
        Self::new(symbol, FuncOrDataId::Func(id))
    }

    pub fn new_data(symbol: String, id: DataId) -> Self {
        Self::new(symbol, FuncOrDataId::Data(id))
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn as_funcid(&self) -> Option<FuncId> {
        match self.id {
            FuncOrDataId::Func(id) => Some(id),
            FuncOrDataId::Data(_) => None,
        }
    }

    pub fn as_dataid(&self) -> Option<DataId> {
        match self.id {
            FuncOrDataId::Data(id) => Some(id),
            FuncOrDataId::Func(_) => None,
        }
    }

    pub fn as_externalname(&self) -> ExternalName {
        ExternalName::from(self.id)
    }
}

impl From<Name> for ExternalName {
    fn from(name: Name) -> ExternalName {
        ExternalName::from(name.id)
    }
}
