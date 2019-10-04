use crate::AtomType;
use witx::CoreFuncType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncSignature {
    pub args: Vec<AtomType>,
    pub ret: Option<AtomType>,
}

impl From<CoreFuncType> for FuncSignature {
    fn from(m: CoreFuncType) -> FuncSignature {
        FuncSignature {
            args: m.args.iter().map(|a| a.repr()).collect(),
            ret: m.ret.map(|r| r.repr()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFunc {
    pub module: String,
    pub field: String,
    pub ty: FuncSignature,
}
