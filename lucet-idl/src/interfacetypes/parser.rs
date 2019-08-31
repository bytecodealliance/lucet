#![allow(unreachable_code)] // wip
#![allow(unused_variables)] // wip
#![allow(dead_code)] // wip
use super::sexpr::SExpr;

type ParseError = (); // TODO

#[derive(Debug, Clone, Copy)]
pub enum BuiltinType {
    String,
    U8,
    U16,
    U32,
    U64,
}

impl BuiltinType {
    pub fn parse<'a>(sexpr: &SExpr<'a>) -> Result<Self, ParseError> {
        match sexpr {
            SExpr::Word("string", _loc) => Ok(BuiltinType::String),
            SExpr::Word("u8", _loc) => Ok(BuiltinType::U8),
            SExpr::Word("u16", _loc) => Ok(BuiltinType::U16),
            SExpr::Word("u32", _loc) => Ok(BuiltinType::U32),
            SExpr::Word("u64", _loc) => Ok(BuiltinType::U64),
            _ => Err(unimplemented!()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeIdent<'a> {
    Builtin(BuiltinType),
    Array(Box<TypeIdent<'a>>),
    Ident(&'a str),
}

impl<'a> TypeIdent<'a> {
    pub fn parse(sexpr: &SExpr<'a>) -> Result<TypeIdent<'a>, ParseError> {
        if let Ok(builtin) = BuiltinType::parse(sexpr) {
            Ok(TypeIdent::Builtin(builtin))
        } else {
            match sexpr {
                SExpr::Ident(i, _loc) => Ok(TypeIdent::Ident(i)),
                SExpr::Vec(v, _loc) => {
                    if v.len() == 2 && v[0].is_word("array") {
                        Ok(TypeIdent::Array(Box::new(TypeIdent::parse(&v[1])?)))
                    } else {
                        Err(unimplemented!("expected type constructor"))
                    }
                }
                _ => Err(unimplemented!("expected type identifier")),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeclSyntax<'a> {
    Typename(TypenameSyntax<'a>),
    Moduletype(ModuletypeSyntax<'a>),
}

impl<'a> DeclSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<DeclSyntax<'a>, ParseError> {
        match sexpr {
            SExpr::Vec(v, _loc) if v.len() > 1 => match v[0] {
                SExpr::Word("typename", _loc) => {
                    Ok(DeclSyntax::Typename(TypenameSyntax::parse(&v[1..])?))
                }
                SExpr::Word("moudletype", _loc) => {
                    Ok(DeclSyntax::Moduletype(ModuletypeSyntax::parse(&v[1..])?))
                }
                _ => Err(unimplemented!("invalid declaration")),
            },
            _ => Err(unimplemented!("expected vec")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypenameSyntax<'a> {
    pub ident: &'a str,
    pub def: TypedefSyntax<'a>,
}

impl<'a> TypenameSyntax<'a> {
    fn parse(sexpr: &[SExpr<'a>]) -> Result<TypenameSyntax<'a>, ParseError> {
        let ident = match sexpr.get(0) {
            Some(SExpr::Ident(i, _)) => i,
            _ => Err(unimplemented!("expected typename identifier"))?,
        };
        let def = match sexpr.get(1) {
            Some(SExpr::Vec(v, _)) => TypedefSyntax::parse(v)?,
            _ => Err(panic!("expected type definition"))?,
        };
        Ok(TypenameSyntax { ident, def })
    }
}

#[derive(Debug, Clone)]
pub enum TypedefSyntax<'a> {
    Enum(EnumSyntax<'a>),
    Flags(FlagsSyntax<'a>),
    Struct(StructSyntax<'a>),
}

impl<'a> TypedefSyntax<'a> {
    fn parse(sexpr: &[SExpr<'a>]) -> Result<TypedefSyntax<'a>, ParseError> {
        match sexpr.get(0) {
            Some(SExpr::Word("enum", _)) => {
                Ok(TypedefSyntax::Enum(EnumSyntax::parse(&sexpr[1..])?))
            }
            Some(SExpr::Word("flags", _)) => {
                Ok(TypedefSyntax::Flags(FlagsSyntax::parse(&sexpr[1..])?))
            }
            Some(SExpr::Word("struct", _)) => {
                Ok(TypedefSyntax::Struct(StructSyntax::parse(&sexpr[1..])?))
            }
            _ => Err(panic!("invalid typedef")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumSyntax<'a> {
    pub repr: BuiltinType,
    pub members: Vec<&'a str>,
}

impl<'a> EnumSyntax<'a> {
    fn parse(sexpr: &[SExpr<'a>]) -> Result<EnumSyntax<'a>, ParseError> {
        let repr = match sexpr.get(0) {
            Some(e) => BuiltinType::parse(e)?,
            _ => Err(panic!("no enum repr"))?,
        };
        let members = match sexpr.get(1) {
            Some(SExpr::Vec(v, _)) => v
                .iter()
                .map(|m| match m {
                    SExpr::Ident(i, _) => Ok(*i),
                    _ => Err(panic!("expected enum member identifier")),
                })
                .collect::<Result<Vec<_>, ParseError>>()?,
            _ => Err(panic!("empty enum members"))?,
        };
        Ok(EnumSyntax { repr, members })
    }
}

#[derive(Debug, Clone)]
pub struct FlagsSyntax<'a> {
    pub repr: BuiltinType,
    pub flags: Vec<&'a str>,
}

impl<'a> FlagsSyntax<'a> {
    fn parse(sexpr: &[SExpr<'a>]) -> Result<FlagsSyntax<'a>, ParseError> {
        let repr = BuiltinType::parse(
            sexpr
                .get(0)
                .ok_or_else(|| panic!("expected flag repr type"))?,
        )?;
        let flags = sexpr[1..]
            .iter()
            .map(|f| match f {
                SExpr::Ident(i, _) => Ok(*i),
                _ => Err(panic!("expected flag identifier")),
            })
            .collect::<Result<Vec<_>, ParseError>>()?;
        Ok(FlagsSyntax { repr, flags })
    }
}

#[derive(Debug, Clone)]
pub struct StructSyntax<'a> {
    pub fields: Vec<StructFieldSyntax<'a>>,
}

impl<'a> StructSyntax<'a> {
    fn parse(sexpr: &[SExpr<'a>]) -> Result<StructSyntax<'a>, ParseError> {
        let fields = sexpr
            .iter()
            .map(|f| StructFieldSyntax::parse(f))
            .collect::<Result<Vec<_>, ParseError>>()?;
        Ok(StructSyntax { fields })
    }
}

#[derive(Debug, Clone)]
pub struct StructFieldSyntax<'a> {
    pub name: &'a str,
    pub type_: TypeIdent<'a>,
}

impl<'a> StructFieldSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<StructFieldSyntax<'a>, ParseError> {
        match sexpr {
            SExpr::Vec(v, _) => match v.get(0) {
                Some(SExpr::Word("field", _)) => {
                    let name = match v.get(1) {
                        Some(SExpr::Ident(i, _)) => i,
                        _ => Err(panic!("expected struct name identifier"))?,
                    };
                    let type_ = TypeIdent::parse(
                        v.get(2)
                            .ok_or_else(|| panic!("expected struct type identifier"))?,
                    )?;
                    Ok(StructFieldSyntax { name, type_ })
                }
                _ => Err(panic!("expected struct field")),
            },
            _ => Err(panic!("expected struct field vector")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuletypeSyntax<'a> {
    pub imports: Vec<ModuleImportSyntax<'a>>,
    pub funcs: Vec<ModuleFuncSyntax<'a>>,
}

impl<'a> ModuletypeSyntax<'a> {
    fn parse(sexprs: &[SExpr<'a>]) -> Result<ModuletypeSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleImportSyntax<'a> {
    pub name: &'a str,
    pub type_: ImportTypeSyntax<'a>,
}

impl<'a> ModuleImportSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<ModuleImportSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub enum ImportTypeSyntax<'a> {
    Memory,
    Func(FunctionTypeSyntax<'a>),
}

impl<'a> ImportTypeSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<ImportTypeSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionTypeSyntax<'a> {
    pub params: Vec<TypeIdent<'a>>,
    pub results: Vec<TypeIdent<'a>>,
}

impl<'a> FunctionTypeSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<FunctionTypeSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct ParamSyntax<'a> {
    pub name: &'a str,
    pub type_: TypeIdent<'a>,
}

impl<'a> ParamSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<ParamSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleFuncSyntax<'a> {
    pub export: &'a str,
    pub params: Vec<ParamSyntax<'a>>,
    pub results: Vec<ParamSyntax<'a>>,
}

impl<'a> ModuleFuncSyntax<'a> {
    fn parse(sexpr: &SExpr<'a>) -> Result<ModuleFuncSyntax<'a>, ParseError> {
        unimplemented!()
    }
}
