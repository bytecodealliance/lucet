use lucet_idl::{AbiType, AtomType};
use proptest::prelude::*;

pub trait ArbTypeExt
where
    Self: Sized,
{
    fn strat() -> BoxedStrategy<Self>;
    fn render_idl(&self) -> String;
}

impl ArbTypeExt for AtomType {
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

impl ArbTypeExt for AbiType {
    fn strat() -> BoxedStrategy<AbiType> {
        prop_oneof![
            Just(AbiType::I32),
            Just(AbiType::I64),
            Just(AbiType::F32),
            Just(AbiType::F64),
        ]
        .boxed()
    }
    fn render_idl(&self) -> String {
        use AbiType::*;
        match self {
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

    pub fn normalize(self, highest_definition: usize) -> Self {
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
    pub fn normalize(self, highest_definition: usize) -> Self {
        match self {
            DatatypeName::Defined(def) => {
                if highest_definition == 0 {
                    // No defined type to normalize to - instead use an atom type.
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
        // up to 20 variants for now. probably want to allow more in the future?
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

    pub fn normalize(self, highest_definition: usize) -> Self {
        let members = self
            .members
            .into_iter()
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
    pub fn normalize(self, highest_definition: usize) -> Self {
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

    pub fn normalize(self, highest_definition: usize) -> Self {
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
pub enum FuncArgBindingSyntax {
    InValue(AtomType),
    InPtr(DatatypeRef),
    OutPtr(DatatypeRef),
    InOutPtr(DatatypeRef),
    InSlice(DatatypeRef),
    InOutSlice(DatatypeRef),
}

impl FuncArgBindingSyntax {
    pub fn strat() -> BoxedStrategy<FuncArgBindingSyntax> {
        use FuncArgBindingSyntax::*;
        prop_oneof![
            AtomType::strat().prop_map(InValue),
            DatatypeRef::strat().prop_map(InPtr),
            DatatypeRef::strat().prop_map(OutPtr),
            DatatypeRef::strat().prop_map(InOutPtr),
            DatatypeRef::strat().prop_map(InSlice),
            DatatypeRef::strat().prop_map(InOutSlice),
        ]
        .boxed()
    }

    pub fn normalize(self, highest_definition: usize) -> Self {
        use FuncArgBindingSyntax::*;
        match self {
            InValue(atomtype) => InValue(atomtype),
            InPtr(dt) => InPtr(dt.normalize(highest_definition)),
            OutPtr(dt) => OutPtr(dt.normalize(highest_definition)),
            InOutPtr(dt) => InOutPtr(dt.normalize(highest_definition)),
            InSlice(dt) => InSlice(dt.normalize(highest_definition)),
            InOutSlice(dt) => InOutSlice(dt.normalize(highest_definition)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionSyntax {
    arg_bindings: Vec<FuncArgBindingSyntax>,
    ret_binding: Option<AtomType>,
}

impl FunctionSyntax {
    pub fn strat(max_args: usize) -> impl Strategy<Value = Self> {
        (
            prop::collection::vec(FuncArgBindingSyntax::strat(), 0..max_args),
            prop::option::of(AtomType::strat()),
        )
            .prop_map(|(arg_bindings, ret_binding)| FunctionSyntax {
                arg_bindings,
                ret_binding,
            })
    }

    pub fn normalize(self, highest_definition: usize) -> Self {
        let arg_bindings = self
            .arg_bindings
            .into_iter()
            .map(|a| a.normalize(highest_definition))
            .collect();
        Self {
            arg_bindings,
            ret_binding: self.ret_binding,
        }
    }

    pub fn render_idl(&self, name: usize) -> String {
        let mut arg_syntax: Vec<String> = Vec::new();
        let mut binding_syntax: Vec<String> = Vec::new();

        for (ix, a) in self.arg_bindings.iter().enumerate() {
            use FuncArgBindingSyntax::*;
            match a {
                InValue(atomtype) => {
                    arg_syntax.push(format!(
                        "a_{}: {}",
                        ix,
                        AbiType::smallest_representation(atomtype).render_idl()
                    ));
                    binding_syntax.push(format!(
                        "b_{}: in {} <- a_{}",
                        ix,
                        atomtype.render_idl(),
                        ix
                    ));
                }
                InPtr(dt) => {
                    arg_syntax.push(format!("a_{}: i32", ix));
                    binding_syntax.push(format!("b_{}: in {} <- *a_{}", ix, dt.render_idl(), ix));
                }
                OutPtr(dt) => {
                    arg_syntax.push(format!("a_{}: i32", ix));
                    binding_syntax.push(format!("b_{}: out {} <- *a_{}", ix, dt.render_idl(), ix));
                }
                InOutPtr(dt) => {
                    arg_syntax.push(format!("a_{}: i32", ix));
                    binding_syntax.push(format!(
                        "b_{}: inout {} <- *a_{}",
                        ix,
                        dt.render_idl(),
                        ix
                    ));
                }
                InSlice(dt) => {
                    arg_syntax.push(format!("a_{}_ptr: i32", ix));
                    arg_syntax.push(format!("a_{}_len: i32", ix));
                    binding_syntax.push(format!(
                        "b_{}: in {} <- [a_{}_ptr, a_{}_len]",
                        ix,
                        dt.render_idl(),
                        ix,
                        ix
                    ));
                }
                InOutSlice(dt) => {
                    arg_syntax.push(format!("a_{}_ptr: i32", ix));
                    arg_syntax.push(format!("a_{}_len: i32", ix));
                    binding_syntax.push(format!(
                        "b_{}: inout {} <- [a_{}_ptr, a_{}_len]",
                        ix,
                        dt.render_idl(),
                        ix,
                        ix
                    ));
                }
            }
        }

        let mut ret_syntax = None;
        if let Some(b) = self.ret_binding {
            ret_syntax = Some(format!(
                "r: {}",
                AbiType::smallest_representation(&b).render_idl()
            ));
            let ix = self.arg_bindings.len();
            binding_syntax.push(format!("b_{}: out {} <- r", ix, b.render_idl(),));
        }

        format!(
            "fn f_{}({}){}\nwhere {};",
            name,
            arg_syntax.join(", "),
            ret_syntax.map(|r| format!("-> {}", r)).unwrap_or_default(),
            binding_syntax.join(",\n"),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub datatype_decls: Vec<DatatypeSyntax>,
    pub function_decls: Vec<FunctionSyntax>,
}

impl Spec {
    pub fn strat(max_size: usize) -> impl Strategy<Value = Self> {
        (
            prop::collection::vec(DatatypeSyntax::strat(), 1..max_size),
            prop::collection::vec(FunctionSyntax::strat(max_size), 1..max_size),
        )
            .prop_map(|(ds, fs)| Self::from_decls(ds, fs))
    }

    pub fn from_decls(
        datatype_decls: Vec<DatatypeSyntax>,
        function_decls: Vec<FunctionSyntax>,
    ) -> Self {
        let datatype_decls: Vec<DatatypeSyntax> = datatype_decls
            .into_iter()
            .enumerate()
            .map(|(ix, decl)| decl.normalize(ix))
            .collect();
        let num_datatypes = datatype_decls.len();
        let function_decls = function_decls
            .into_iter()
            .map(|decl| decl.normalize(num_datatypes))
            .collect();
        Spec {
            datatype_decls,
            function_decls,
        }
    }

    pub fn render_idl(&self) -> String {
        let mut s = String::new();
        for (ix, d) in self.datatype_decls.iter().enumerate() {
            s += &format!("    {}\n", d.render_idl(ix));
        }
        for (ix, d) in self.function_decls.iter().enumerate() {
            s += &format!("    {}\n", d.render_idl(ix));
        }
        format!("mod spec {{\n{}\n}}", s)
    }
}
