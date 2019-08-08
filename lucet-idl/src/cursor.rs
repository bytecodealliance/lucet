pub use crate::atoms::{AbiType, AtomType};
use crate::repr::{
    AliasDatatypeRepr, BindingFromRepr, BindingIx, BindingRepr, DatatypeIdent, DatatypeIx,
    DatatypeRepr, DatatypeVariantRepr, EnumDatatypeRepr, FuncIdent, FuncIx, FuncRepr, ModuleIx,
    ModuleRepr, ParamIx, ParamRepr, StructDatatypeRepr, StructMemberRepr,
};
pub use crate::repr::{BindingDirection, Package};
use crate::MemArea;
use std::convert::TryFrom;
use std::ops::Deref;

impl Package {
    pub fn module<'a>(&'a self, name: &str) -> Option<Module<'a>> {
        self.names
            .iter()
            .find(|(_, n)| *n == name)
            .and_then(|(ix, _)| self.module_by_ix(ix))
    }

    fn all_modules<'a>(&'a self) -> impl Iterator<Item = Module<'a>> + 'a {
        self.names.keys().map(move |ix| Module { pkg: &self, ix })
    }

    pub fn modules<'a>(&'a self) -> impl Iterator<Item = Module<'a>> + 'a {
        self.all_modules().filter(|m| m.name() != "std")
    }

    pub fn module_by_ix<'a>(&'a self, ix: ModuleIx) -> Option<Module<'a>> {
        if self.modules.is_valid(ix) {
            Some(Module { pkg: &self, ix })
        } else {
            None
        }
    }

    pub fn datatype_by_id<'a>(&'a self, id: DatatypeIdent) -> Option<Datatype<'a>> {
        self.module_by_ix(id.module)
            .and_then(|m| m.datatype_by_ix(id.datatype))
    }
}

#[derive(Debug, Clone)]
pub struct Module<'a> {
    pkg: &'a Package,
    ix: ModuleIx,
}

impl<'a> Module<'a> {
    fn repr(&self) -> &'a ModuleRepr {
        self.pkg.modules.get(self.ix).expect("i exist")
    }

    pub fn package(&self) -> &'a Package {
        self.pkg
    }

    pub fn name(&self) -> &str {
        self.pkg.names.get(self.ix).expect("i exist")
    }

    pub fn datatype(&self, name: &str) -> Option<Datatype<'a>> {
        if let Ok(atom) = AtomType::try_from(name) {
            Some(
                self.pkg
                    .datatype_by_id(atom.datatype_id())
                    .expect("atom from id"),
            )
        } else {
            self.repr()
                .datatypes
                .names
                .iter()
                .find(|(_, n)| *n == name)
                .and_then(|(ix, _)| self.datatype_by_ix(ix))
        }
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
    pkg: &'a Package,
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
                datatype: self.clone(),
                repr: &repr,
            }),
            DatatypeVariantRepr::Enum(ref repr) => DatatypeVariant::Enum(EnumDatatype {
                datatype: self.clone(),
                repr: &repr,
            }),
            DatatypeVariantRepr::Alias(ref repr) => DatatypeVariant::Alias(AliasDatatype {
                datatype: self.clone(),
                repr: &repr,
            }),
        }
    }

    pub fn abi_type(&self) -> Option<AbiType> {
        self.variant().abi_type()
    }

    pub fn anti_alias(&self) -> Datatype<'a> {
        match self.repr().variant {
            DatatypeVariantRepr::Alias(ref repr) => AliasDatatype {
                datatype: self.clone(),
                repr: &repr,
            }
            .anti_alias(),
            _ => self.clone(),
        }
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

    pub fn atom(self) -> Option<AtomType> {
        match self {
            DatatypeVariant::Atom(a) => Some(a),
            _ => None,
        }
    }
    pub fn struct_(self) -> Option<StructDatatype<'a>> {
        match self {
            DatatypeVariant::Struct(s) => Some(s),
            _ => None,
        }
    }
    pub fn enum_(self) -> Option<EnumDatatype<'a>> {
        match self {
            DatatypeVariant::Enum(e) => Some(e),
            _ => None,
        }
    }
    pub fn alias(self) -> Option<AliasDatatype<'a>> {
        match self {
            DatatypeVariant::Alias(a) => Some(a),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructDatatype<'a> {
    datatype: Datatype<'a>,
    repr: &'a StructDatatypeRepr,
}

impl<'a> StructDatatype<'a> {
    pub fn member(&self, name: &str) -> Option<StructMember<'a>> {
        self.members().find(|m| m.name() == name)
    }

    pub fn members(&self) -> impl Iterator<Item = StructMember<'a>> {
        let struct_ = self.clone();
        self.repr.members.iter().map(move |repr| StructMember {
            struct_: struct_.clone(),
            repr,
        })
    }
}

impl<'a> Deref for StructDatatype<'a> {
    type Target = Datatype<'a>;
    fn deref(&self) -> &Self::Target {
        &self.datatype
    }
}

impl<'a> From<StructDatatype<'a>> for Datatype<'a> {
    fn from(s: StructDatatype<'a>) -> Datatype<'a> {
        s.datatype.clone()
    }
}

#[derive(Debug, Clone)]
pub struct StructMember<'a> {
    struct_: StructDatatype<'a>,
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
            pkg: self.struct_.datatype.pkg,
            id: self.repr.type_,
        }
    }
    pub fn struct_(&self) -> StructDatatype<'a> {
        self.struct_.clone()
    }
}

#[derive(Debug, Clone)]
pub struct EnumDatatype<'a> {
    datatype: Datatype<'a>,
    repr: &'a EnumDatatypeRepr,
}

impl<'a> EnumDatatype<'a> {
    pub fn variants(&self) -> impl Iterator<Item = EnumVariant<'a>> {
        let enum_ = self.clone();
        (0..self.repr.members.len())
            .into_iter()
            .map(move |ix| EnumVariant {
                enum_: enum_.clone(),
                index: ix,
            })
    }

    pub fn variant(&self, name: &str) -> Option<EnumVariant<'a>> {
        self.variants().find(|v| v.name() == name)
    }
}

impl<'a> Deref for EnumDatatype<'a> {
    type Target = Datatype<'a>;
    fn deref(&self) -> &Self::Target {
        &self.datatype
    }
}

impl<'a> From<EnumDatatype<'a>> for Datatype<'a> {
    fn from(e: EnumDatatype<'a>) -> Datatype<'a> {
        e.datatype
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariant<'a> {
    enum_: EnumDatatype<'a>,
    index: usize,
}

impl<'a> EnumVariant<'a> {
    pub fn enum_(&self) -> EnumDatatype<'a> {
        self.enum_.clone()
    }
    pub fn name(&self) -> &str {
        &self.enum_.repr.members[self.index].name
    }
    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Debug, Clone)]
pub struct AliasDatatype<'a> {
    datatype: Datatype<'a>,
    repr: &'a AliasDatatypeRepr,
}

impl<'a> AliasDatatype<'a> {
    pub fn to(&self) -> Datatype<'a> {
        Datatype {
            pkg: self.datatype.pkg,
            id: self.repr.to,
        }
    }

    /// Find the non-alias datatype that this alias transitively refers to.
    pub fn anti_alias(&self) -> Datatype<'a> {
        // We can't just call this recursively because
        // of the borrow checker, so we have to recurse in a loop :/
        let mut referent = Datatype {
            pkg: self.datatype.pkg,
            id: self.repr.to,
        };
        loop {
            match referent.variant() {
                DatatypeVariant::Alias(a) => {
                    referent.id = a.datatype.id;
                }
                _ => break,
            }
        }
        Datatype {
            pkg: self.datatype.pkg,
            id: referent.id,
        }
    }
}

impl<'a> From<AliasDatatype<'a>> for Datatype<'a> {
    fn from(a: AliasDatatype<'a>) -> Datatype<'a> {
        a.datatype.clone()
    }
}

impl<'a> Deref for AliasDatatype<'a> {
    type Target = Datatype<'a>;
    fn deref(&self) -> &Self::Target {
        &self.datatype
    }
}

#[derive(Debug, Clone)]
pub struct Function<'a> {
    pkg: &'a Package,
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

    pub fn module(&self) -> Module<'a> {
        Module {
            pkg: self.pkg,
            ix: self.id.module,
        }
    }

    pub fn host_func_name(&self) -> String {
        use heck::SnakeCase;
        format!(
            "__{}_{}",
            self.module().name().to_snake_case(),
            self.name().to_snake_case()
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ParamPosition {
    Arg(usize),
    Ret(usize),
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
        self.func
            .pkg
            .datatype_by_id(AtomType::from(self.repr().type_).datatype_id())
            .expect("valid type")
    }
    pub fn param_position(&self) -> ParamPosition {
        match self.ix {
            ParamIx::Arg(ix) => ParamPosition::Arg(ix.as_u32() as usize),
            ParamIx::Ret(ix) => ParamPosition::Ret(ix.as_u32() as usize),
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
        self.func
            .pkg
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

impl<'a> BindingParam<'a> {
    pub fn ptr(self) -> Option<FuncParam<'a>> {
        match self {
            BindingParam::Ptr(p) => Some(p),
            _ => None,
        }
    }
    pub fn slice(self) -> Option<(FuncParam<'a>, FuncParam<'a>)> {
        match self {
            BindingParam::Slice(p, l) => Some((p, l)),
            _ => None,
        }
    }
    pub fn value(self) -> Option<FuncParam<'a>> {
        match self {
            BindingParam::Value(v) => Some(v),
            _ => None,
        }
    }
}
