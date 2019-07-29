use crate::env::memarea::MemArea;
pub use crate::env::types::EnumMember;
use crate::env::types::{
    AliasDatatypeRepr, AtomType, DatatypeIdent, DatatypeRepr, DatatypeVariantRepr,
    EnumDatatypeRepr, ModuleIx, ModuleRepr, PackageRepr, StructDatatypeRepr, StructMemberRepr,
};

pub struct Package {
    repr: PackageRepr,
}

impl Package {
    pub fn module<'a>(&'a self, name: &str) -> Option<Module<'a>> {
        if let Some((ix, _)) = self.repr.names.iter().find(|(_, n)| *n == name) {
            Some(Module {
                pkg: &self.repr,
                ix,
            })
        } else {
            None
        }
    }
}

pub struct Module<'a> {
    pkg: &'a PackageRepr,
    ix: ModuleIx,
}

impl<'a> Module<'a> {
    fn repr(&self) -> &'a ModuleRepr {
        self.pkg.modules.get(self.ix).expect("i exist")
    }

    pub fn name(&self) -> &str {
        self.pkg.names.get(self.ix).expect("i exist")
    }

    pub fn datatype(&self, name: &str) -> Option<Datatype<'a>> {
        if let Some((ix, _)) = self.repr().datatypes.names.iter().find(|(_, n)| *n == name) {
            Some(Datatype {
                pkg: self.pkg,
                id: DatatypeIdent::new(self.ix, ix),
            })
        } else {
            None
        }
    }

    pub fn datatypes(&self) -> impl Iterator<Item = Datatype<'a>> {
        let pkg = self.pkg;
        let mix = self.ix;
        self.repr()
            .datatypes
            .datatypes
            .keys()
            .map(move |ix| Datatype {
                pkg,
                id: DatatypeIdent::new(mix, ix),
            })
    }
}

pub struct Datatype<'a> {
    pkg: &'a PackageRepr,
    id: DatatypeIdent,
}

impl<'a> Datatype<'a> {
    fn repr(&self) -> &'a DatatypeRepr {
        self.pkg
            .modules
            .get(self.id.module)
            .expect("my mod exists")
            .datatypes
            .datatypes
            .get(self.id.datatype)
            .expect("i exist")
    }

    pub fn name(&self) -> &'a str {
        self.pkg
            .modules
            .get(self.id.module)
            .expect("my mod exists")
            .datatypes
            .names
            .get(self.id.datatype)
            .expect("i exist")
    }

    pub fn variant(&'a self) -> DatatypeVariant<'a> {
        match self.repr().variant {
            DatatypeVariantRepr::Atom(a) => DatatypeVariant::Atom(a),
            DatatypeVariantRepr::Struct(ref repr) => DatatypeVariant::Struct(StructDatatype {
                pkg: self.pkg,
                repr: &repr,
                id: self.id,
            }),
            DatatypeVariantRepr::Enum(ref repr) => DatatypeVariant::Enum(EnumDatatype {
                pkg: self.pkg,
                repr: &repr,
                id: self.id,
            }),
            DatatypeVariantRepr::Alias(ref repr) => DatatypeVariant::Alias(AliasDatatype {
                pkg: self.pkg,
                repr: &repr,
                id: self.id,
            }),
        }
    }
}

impl<'a> MemArea for Datatype<'a> {
    fn repr_size(&self) -> usize {
        self.repr().repr_size
    }
    fn align(&self) -> usize {
        self.repr().align
    }
}

#[derive(Debug, Clone)]
pub enum DatatypeVariant<'a> {
    Atom(AtomType),
    Struct(StructDatatype<'a>),
    Enum(EnumDatatype<'a>),
    Alias(AliasDatatype<'a>),
}

#[derive(Debug, Clone)]
pub struct StructDatatype<'a> {
    pkg: &'a PackageRepr,
    repr: &'a StructDatatypeRepr,
    id: DatatypeIdent,
}

impl<'a> StructDatatype<'a> {
    pub fn name(&self) -> &str {
        Datatype {
            pkg: self.pkg,
            id: self.id,
        }
        .name()
    }
    pub fn members(&self) -> impl Iterator<Item = StructMember<'a>> {
        let pkg = self.pkg;
        self.repr
            .members
            .iter()
            .map(move |repr| StructMember { pkg, repr })
    }
}

impl<'a> From<StructDatatype<'a>> for Datatype<'a> {
    fn from(s: StructDatatype<'a>) -> Datatype<'a> {
        Datatype {
            pkg: s.pkg,
            id: s.id,
        }
    }
}

pub struct StructMember<'a> {
    pkg: &'a PackageRepr,
    repr: &'a StructMemberRepr,
}

impl<'a> StructMember<'a> {
    pub fn name(&self) -> &str {
        &self.repr.name
    }
    pub fn offset(&self) -> usize {
        self.repr.offset
    }
    pub fn type_(&self) -> Datatype<'a> {
        Datatype {
            pkg: self.pkg,
            id: self.repr.type_,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumDatatype<'a> {
    pkg: &'a PackageRepr,
    repr: &'a EnumDatatypeRepr,
    id: DatatypeIdent,
}

impl<'a> EnumDatatype<'a> {
    pub fn name(&self) -> &str {
        Datatype {
            pkg: self.pkg,
            id: self.id,
        }
        .name()
    }
    pub fn members(&self) -> &'a [EnumMember] {
        &self.repr.members
    }
}
impl<'a> From<EnumDatatype<'a>> for Datatype<'a> {
    fn from(e: EnumDatatype<'a>) -> Datatype<'a> {
        Datatype {
            pkg: e.pkg,
            id: e.id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AliasDatatype<'a> {
    pkg: &'a PackageRepr,
    repr: &'a AliasDatatypeRepr,
    id: DatatypeIdent,
}

impl<'a> AliasDatatype<'a> {
    pub fn name(&self) -> &str {
        Datatype {
            pkg: self.pkg,
            id: self.id,
        }
        .name()
    }
    pub fn to(&self) -> Datatype<'a> {
        Datatype {
            pkg: self.pkg,
            id: self.repr.to,
        }
    }
}

impl<'a> From<AliasDatatype<'a>> for Datatype<'a> {
    fn from(a: AliasDatatype<'a>) -> Datatype<'a> {
        Datatype {
            pkg: a.pkg,
            id: a.id,
        }
    }
}
