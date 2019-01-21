#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AtomType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Attr {
    pub key: String,
    pub val: String,
    pub location: Location,
}

impl Attr {
    pub fn new(key: &str, val: &str, location: Location) -> Attr {
        Attr {
            key: key.to_owned(),
            val: val.to_owned(),
            location,
        }
    }
}
