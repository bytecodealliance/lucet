pub use wasmparser::Type as AtomType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncSignature {
    pub params: Vec<AtomType>,
    pub results: Vec<AtomType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFunc {
    pub module: String,
    pub field: String,
    pub ty: FuncSignature,
}
