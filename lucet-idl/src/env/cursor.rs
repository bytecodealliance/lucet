use crate::env::atoms::{AbiType, AtomType};
pub use crate::env::repr::BindingDirection;
use crate::env::repr::{
    AliasDatatypeRepr, BindingFromRepr, BindingIx, BindingRepr, DatatypeIdent, DatatypeIx,
    DatatypeRepr, DatatypeVariantRepr, EnumDatatypeRepr, FuncIdent, FuncIx, FuncRepr, ModuleIx,
    ModuleRepr, PackageRepr, ParamIx, ParamRepr, StructDatatypeRepr, StructMemberRepr,
};
use crate::env::MemArea;
use crate::parser::SyntaxTypeRef;

#[derive(Debug, Clone)]
pub struct Package<'a> {
    repr: &'a PackageRepr,
}

impl<'a> Package<'a> {
    pub fn new(repr: &'a PackageRepr) -> Package<'a> {
        Package { repr }
    }

    pub fn module(&self, name: &str) -> Option<Module<'a>> {
        self.repr
            .names
            .iter()
            .find(|(_, n)| *n == name)
            .and_then(|(ix, _)| self.module_by_ix(ix))
    }

    pub fn modules(&self) -> impl Iterator<Item = Module<'a>> {
        let pkg = self.repr;
        self.repr.names.keys().map(move |ix| Module { pkg, ix })
    }

    pub fn module_by_ix(&self, ix: ModuleIx) -> Option<Module<'a>> {
        if self.repr.modules.is_valid(ix) {
            Some(Module {
                pkg: &self.repr,
                ix,
            })
        } else {
            None
        }
    }

    pub fn datatype_by_id(&self, id: DatatypeIdent) -> Option<Datatype<'a>> {
        self.module_by_ix(id.module)
            .and_then(|m| m.datatype_by_ix(id.datatype))
    }
}

#[derive(Debug, Clone)]
pub struct Module<'a> {
    pkg: &'a PackageRepr,
    ix: ModuleIx,
}

impl<'a> Module<'a> {
    fn repr(&self) -> &'a ModuleRepr {
        self.pkg.modules.get(self.ix).expect("i exist")
    }

    pub fn package(&self) -> Package<'a> {
        Package { repr: self.pkg }
    }

    pub fn name(&self) -> &str {
        self.pkg.names.get(self.ix).expect("i exist")
    }

    pub fn datatype(&self, name: &str) -> Option<Datatype<'a>> {
        self.repr()
            .datatypes
            .names
            .iter()
            .find(|(_, n)| *n == name)
            .and_then(|(ix, _)| self.datatype_by_ix(ix))
    }

    pub fn datatype_by_ix(&self, ix: DatatypeIx) -> Option<Datatype<'a>> {
        if self.repr().datatypes.datatypes.is_valid(ix) {
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

    // XXX move this to a trait that we dont export, eventaully...
    pub fn datatype_by_syntax(&self, tref: &SyntaxTypeRef) -> Option<Datatype<'a>> {
        match tref {
            SyntaxTypeRef::Name { name, .. } => self.datatype(name),
            SyntaxTypeRef::Atom { atom, .. } => self.package().datatype_by_id(atom.datatype_id()),
        }
    }

    pub fn function(&self, name: &str) -> Option<Function<'a>> {
        self.repr()
            .funcs
            .names
            .iter()
            .find(|(_, n)| *n == name)
            .and_then(|(ix, _)| self.function_by_ix(ix))
    }

    pub fn function_by_ix(&self, ix: FuncIx) -> Option<Function<'a>> {
        if self.repr().funcs.funcs.is_valid(ix) {
            Some(Function {
                pkg: self.pkg,
                id: FuncIdent::new(self.ix, ix),
            })
        } else {
            None
        }
    }

    pub fn functions(&self) -> impl Iterator<Item = Function<'a>> {
        let pkg = self.pkg;
        let mix = self.ix;
        self.repr().funcs.funcs.keys().map(move |ix| Function {
            pkg,
            id: FuncIdent::new(mix, ix),
        })
    }
}

#[derive(Debug, Clone)]
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

    pub fn id(&self) -> DatatypeIdent {
        self.id
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

    pub fn abi_type(&self) -> Option<AbiType> {
        self.variant().abi_type()
    }
}

impl<'a> MemArea for Datatype<'a> {
    fn mem_size(&self) -> usize {
        self.repr().mem_size
    }
    fn mem_align(&self) -> usize {
        self.repr().mem_align
    }
}

#[derive(Debug, Clone)]
pub enum DatatypeVariant<'a> {
    Atom(AtomType),
    Struct(StructDatatype<'a>),
    Enum(EnumDatatype<'a>),
    Alias(AliasDatatype<'a>),
}

impl<'a> DatatypeVariant<'a> {
    pub fn abi_type(&self) -> Option<AbiType> {
        match self {
            DatatypeVariant::Atom(a) => Some(AbiType::from_atom(a)),
            DatatypeVariant::Struct(_) => None,
            DatatypeVariant::Enum(_) => Some(AbiType::I32),
            DatatypeVariant::Alias(a) => a.to().abi_type(),
        }
    }
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
    pub fn member(&self, name: &str) -> Option<StructMember<'a>> {
        let pkg = self.pkg;
        self.repr
            .members
            .iter()
            .find(|m| m.name == name)
            .map(move |repr| StructMember { pkg, repr })
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

#[derive(Debug, Clone)]
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
    pub fn variants(&self) -> impl Iterator<Item = EnumMember<'a>> {
        let repr = self.clone();
        (0..self.repr.members.len())
            .into_iter()
            .map(move |ix| EnumMember {
                repr: repr.clone(),
                index: ix,
            })
    }

    pub fn variant(&self, name: &str) -> Option<EnumMember<'a>> {
        self.variants().find(|v| v.name() == name)
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

pub struct EnumMember<'a> {
    repr: EnumDatatype<'a>,
    index: usize,
}

impl<'a> EnumMember<'a> {
    pub fn parent(&self) -> EnumDatatype<'a> {
        self.repr.clone()
    }
    pub fn name(&self) -> &str {
        &self.repr.repr.members[self.index].name
    }
    pub fn value(&self) -> usize {
        self.index
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

#[derive(Debug, Clone)]
pub struct Function<'a> {
    pkg: &'a PackageRepr,
    id: FuncIdent,
}

impl<'a> Function<'a> {
    fn repr(&self) -> &'a FuncRepr {
        &self.pkg.modules[self.id.module].funcs.funcs[self.id.func]
    }

    pub fn name(&self) -> &str {
        &self.pkg.modules[self.id.module].funcs.names[self.id.func]
    }

    pub fn arg(&self, name: &str) -> Option<FuncParam<'a>> {
        let func = self.clone();
        self.repr()
            .args
            .iter()
            .find(|(_, param)| param.name == name)
            .map(move |(ix, _)| FuncParam {
                func,
                ix: ParamIx::Arg(ix),
            })
    }

    pub fn args(&self) -> impl Iterator<Item = FuncParam<'a>> {
        let func = self.clone();
        self.repr().args.iter().map(move |(ix, _)| FuncParam {
            func: func.clone(),
            ix: ParamIx::Arg(ix),
        })
    }

    pub fn ret(&self, name: &str) -> Option<FuncParam<'a>> {
        let func = self.clone();
        self.repr()
            .rets
            .iter()
            .find(|(_, param)| param.name == name)
            .map(move |(ix, _)| FuncParam {
                func,
                ix: ParamIx::Ret(ix),
            })
    }

    pub fn rets(&self) -> impl Iterator<Item = FuncParam<'a>> {
        let func = self.clone();
        self.repr().rets.iter().map(move |(ix, _)| FuncParam {
            func: func.clone(),
            ix: ParamIx::Ret(ix),
        })
    }

    pub fn param(&self, name: &str) -> Option<FuncParam<'a>> {
        self.arg(name).or_else(|| self.ret(name))
    }

    pub fn params(&self) -> impl Iterator<Item = FuncParam<'a>> {
        self.args().chain(self.rets())
    }

    pub fn binding(&self, name: &str) -> Option<FuncBinding<'a>> {
        let func = self.clone();
        self.repr()
            .bindings
            .iter()
            .find(|(_, bind)| bind.name == name)
            .map(move |(ix, _)| FuncBinding { func, ix })
    }

    pub fn bindings(&self) -> impl Iterator<Item = FuncBinding<'a>> {
        let func = self.clone();
        self.repr().bindings.iter().map(move |(ix, _)| FuncBinding {
            func: func.clone(),
            ix,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    Arg,
    Ret,
}

#[derive(Debug, Clone)]
pub struct FuncParam<'a> {
    func: Function<'a>,
    ix: ParamIx,
}

impl<'a> FuncParam<'a> {
    fn repr(&self) -> &'a ParamRepr {
        match self.ix {
            ParamIx::Arg(ix) => &self.func.repr().args[ix],
            ParamIx::Ret(ix) => &self.func.repr().rets[ix],
        }
    }
    pub fn name(&self) -> &str {
        &self.repr().name
    }
    pub fn abi_type(&self) -> AbiType {
        self.repr().type_
    }
    pub fn type_(&self) -> Datatype<'a> {
        Package::new(self.func.pkg)
            .datatype_by_id(AtomType::from(self.repr().type_).datatype_id())
            .expect("valid type")
    }
    pub fn param_type(&self) -> ParamType {
        match self.ix {
            ParamIx::Arg { .. } => ParamType::Arg,
            ParamIx::Ret { .. } => ParamType::Ret,
        }
    }
    pub fn binding(&self) -> FuncBinding<'a> {
        let func = self.func.clone();
        self.func
            .repr()
            .bindings
            .iter()
            .find(|(_ix, b)| match b.from {
                BindingFromRepr::Ptr(ix) | BindingFromRepr::Value(ix) => ix == self.ix,
                BindingFromRepr::Slice(ptr_ix, len_ix) => ptr_ix == self.ix || len_ix == self.ix,
            })
            .map(|(ix, _)| FuncBinding { func, ix })
            .expect("must be a binding for param")
    }
}

#[derive(Debug, Clone)]
pub struct FuncBinding<'a> {
    func: Function<'a>,
    ix: BindingIx,
}

impl<'a> FuncBinding<'a> {
    fn repr(&self) -> &'a BindingRepr {
        &self.func.repr().bindings[self.ix]
    }
    pub fn name(&self) -> &str {
        &self.repr().name
    }
    pub fn type_(&self) -> Datatype<'a> {
        Package::new(self.func.pkg)
            .datatype_by_id(self.repr().type_)
            .expect("valid type")
    }
    pub fn direction(&self) -> BindingDirection {
        self.repr().direction
    }
    pub fn param(&self) -> BindingParam<'a> {
        match self.repr().from {
            BindingFromRepr::Ptr(ix) => BindingParam::Ptr(FuncParam {
                func: self.func.clone(),
                ix,
            }),
            BindingFromRepr::Slice(ptr_ix, len_ix) => BindingParam::Slice(
                FuncParam {
                    func: self.func.clone(),
                    ix: ptr_ix,
                },
                FuncParam {
                    func: self.func.clone(),
                    ix: len_ix,
                },
            ),
            BindingFromRepr::Value(ix) => BindingParam::Value(FuncParam {
                func: self.func.clone(),
                ix,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BindingParam<'a> {
    Ptr(FuncParam<'a>),
    Slice(FuncParam<'a>, FuncParam<'a>),
    Value(FuncParam<'a>),
}
