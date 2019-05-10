use lucet_idl::AtomType;
use proptest::prelude::*;

pub trait AtomTypeExt
where
    Self: Sized,
{
    fn strat() -> BoxedStrategy<Self>;
    fn render_idl(&self) -> String;
}

impl AtomTypeExt for AtomType {
    fn strat() -> BoxedStrategy<AtomType> {
        prop_oneof![
            Just(AtomType::Bool),
            Just(AtomType::U8),
            Just(AtomType::U16),
            Just(AtomType::U32),
            Just(AtomType::U64),
            Just(AtomType::I8),
            Just(AtomType::I16),
            Just(AtomType::I32),
            Just(AtomType::I64),
            Just(AtomType::F32),
            Just(AtomType::F64),
        ]
        .boxed()
    }
    fn render_idl(&self) -> String {
        use AtomType::*;
        match self {
            Bool => "bool",
            U8 => "u8",
            U16 => "u16",
            U32 => "u32",
            U64 => "u64",
            I8 => "i8",
            I16 => "i16",
            I32 => "i32",
            I64 => "i64",
            F32 => "f32",
            F64 => "f64",
        }
        .to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatatypeRef(usize);

impl DatatypeRef {
    pub fn strat() -> impl Strategy<Value = Self> {
        any::<usize>().prop_map(DatatypeRef)
    }

    pub fn normalize(&self, highest_definition: usize) -> Self {
        assert!(highest_definition != 0);
        DatatypeRef(self.0 % highest_definition)
    }

    pub fn render_idl(&self) -> String {
        format!("dt_{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum DatatypeName {
    Atom(AtomType),
    Defined(DatatypeRef),
}

impl DatatypeName {
    pub fn normalize(&self, highest_definition: usize) -> Self {
        match self {
            DatatypeName::Defined(def) => {
                if highest_definition == 0 {
                    DatatypeName::Atom(AtomType::I64)
                } else {
                    DatatypeName::Defined(def.normalize(highest_definition))
                }
            }
            DatatypeName::Atom(a) => DatatypeName::Atom(a.clone()),
        }
    }

    pub fn strat() -> impl Strategy<Value = Self> {
        prop_oneof![
            DatatypeRef::strat().prop_map(DatatypeName::Defined),
            AtomType::strat().prop_map(DatatypeName::Atom)
        ]
    }

    pub fn render_idl(&self) -> String {
        match self {
            DatatypeName::Atom(a) => a.render_idl(),
            DatatypeName::Defined(d) => d.render_idl(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumSyntax {
    pub variants: usize,
}

impl EnumSyntax {
    pub fn strat() -> impl Strategy<Value = Self> {
        (1..20usize).prop_map(|variants| EnumSyntax { variants })
    }

    pub fn render_idl(&self, name: usize) -> String {
        let mut s = String::new();
        for v in 0..self.variants {
            s += &format!("v_{}, ", v);
        }
        format!("enum dt_{} {{ {} }} ", name, s)
    }
}

#[derive(Debug, Clone)]
pub struct StructSyntax {
    pub members: Vec<DatatypeName>,
}

impl StructSyntax {
    pub fn strat() -> impl Strategy<Value = Self> {
        prop::collection::vec(DatatypeName::strat(), 1..10)
            .prop_map(|members| StructSyntax { members })
    }

    pub fn normalize(&self, highest_definition: usize) -> Self {
        let members = self
            .members
            .iter()
            .map(|m| m.normalize(highest_definition))
            .collect();
        Self { members }
    }

    pub fn render_idl(&self, name: usize) -> String {
        let mut s = String::new();
        for (ix, m) in self.members.iter().enumerate() {
            s += &format!("m_{}: {}, ", ix, m.render_idl());
        }
        format!("struct dt_{} {{ {} }} ", name, s)
    }
}

#[derive(Debug, Clone)]
pub struct AliasSyntax {
    pub target: DatatypeName,
}

impl AliasSyntax {
    pub fn strat() -> impl Strategy<Value = Self> {
        DatatypeName::strat().prop_map(|target| AliasSyntax { target })
    }
    pub fn normalize(&self, highest_definition: usize) -> Self {
        Self {
            target: self.target.normalize(highest_definition),
        }
    }
    pub fn render_idl(&self, name: usize) -> String {
        format!("type dt_{} = {};", name, self.target.render_idl())
    }
}

#[derive(Debug, Clone)]
pub enum DatatypeSyntax {
    Enum(EnumSyntax),
    Struct(StructSyntax),
    Alias(AliasSyntax),
}

impl DatatypeSyntax {
    pub fn strat() -> impl Strategy<Value = Self> {
        prop_oneof![
            EnumSyntax::strat().prop_map(DatatypeSyntax::Enum),
            StructSyntax::strat().prop_map(DatatypeSyntax::Struct),
            AliasSyntax::strat().prop_map(DatatypeSyntax::Alias),
        ]
    }

    pub fn normalize(&self, highest_definition: usize) -> Self {
        match self {
            DatatypeSyntax::Enum(e) => DatatypeSyntax::Enum(e.clone()),
            DatatypeSyntax::Struct(s) => DatatypeSyntax::Struct(s.normalize(highest_definition)),
            DatatypeSyntax::Alias(a) => DatatypeSyntax::Alias(a.normalize(highest_definition)),
        }
    }

    pub fn render_idl(&self, name: usize) -> String {
        match self {
            DatatypeSyntax::Enum(e) => e.render_idl(name),
            DatatypeSyntax::Struct(s) => s.render_idl(name),
            DatatypeSyntax::Alias(a) => a.render_idl(name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub decls: Vec<DatatypeSyntax>,
}

impl Spec {
    pub fn strat(max_size: usize) -> impl Strategy<Value = Self> {
        prop::collection::vec(DatatypeSyntax::strat(), 1..max_size).prop_map(Self::from_decls)
    }

    pub fn from_decls(decls: Vec<DatatypeSyntax>) -> Self {
        let decls = decls
            .iter()
            .enumerate()
            .map(|(ix, decl)| decl.normalize(ix))
            .collect();
        Spec { decls }
    }

    pub fn render_idl(&self) -> String {
        let mut s = String::new();
        for (ix, d) in self.decls.iter().enumerate() {
            s += &format!("    {}\n", d.render_idl(ix));
        }
        format!("mod spec {{\n{}\n}}", s)
    }
}
