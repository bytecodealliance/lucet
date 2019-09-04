#![allow(unreachable_code)] // wip
#![allow(unused_variables)] // wip
#![allow(dead_code)] // wip
use super::sexpr::SExpr;
use crate::Location;

#[derive(Debug, PartialEq, Eq, Clone, Fail)]
pub enum ParseError {
    #[fail(display = "{} at {:?}", _0, _1)]
    Error(String, Location),
    #[fail(display = "Unimplemented: {} at {:?}", _0, _1)]
    Unimplemented(String, Location),
}

macro_rules! parse_err {
    ($loc:expr, $msg:expr) => {
        ParseError::Error($msg.to_string(), $loc)
    };
    ($loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        ParseError::Error(format!($fmt, $( $arg ),+), $loc)
    };
}
macro_rules! parse_unimp {
    ($loc:expr, $msg:expr) => {
        ParseError::Unimplemented($msg.to_string(), $loc)
    };
}

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
            _ => Err(parse_err!(sexpr.location(), "invalid builtin type")),
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
                SExpr::Vec(v, loc) => {
                    if v.len() == 2 && v[0].is_word("array") {
                        Ok(TypeIdent::Array(Box::new(TypeIdent::parse(&v[1])?)))
                    } else {
                        Err(parse_err!(*loc, "expected type constructor"))
                    }
                }
                _ => Err(parse_err!(sexpr.location(), "expected type identifier")),
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
    pub fn parse(sexpr: &SExpr<'a>) -> Result<DeclSyntax<'a>, ParseError> {
        match sexpr {
            SExpr::Vec(v, loc) if v.len() > 1 => match v[0] {
                SExpr::Word("typename", loc) => {
                    Ok(DeclSyntax::Typename(TypenameSyntax::parse(&v[1..], loc)?))
                }
                SExpr::Word("moudletype", loc) => Ok(DeclSyntax::Moduletype(
                    ModuletypeSyntax::parse(&v[1..], loc)?,
                )),
                _ => Err(parse_err!(*loc, "invalid declaration")),
            },
            _ => Err(parse_err!(sexpr.location(), "expected vec")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypenameSyntax<'a> {
    pub ident: &'a str,
    pub def: TypedefSyntax<'a>,
}

impl<'a> TypenameSyntax<'a> {
    pub fn parse(sexpr: &[SExpr<'a>], loc: Location) -> Result<TypenameSyntax<'a>, ParseError> {
        let ident = match sexpr.get(0) {
            Some(SExpr::Ident(i, loc)) => i,
            _ => Err(parse_unimp!(loc, "expected typename identifier"))?,
        };
        let def = match sexpr.get(1) {
            Some(SExpr::Vec(v, loc)) => TypedefSyntax::parse(v, *loc)?,
            _ => Err(parse_err!(loc, "expected type definition"))?,
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
    pub fn parse(sexpr: &[SExpr<'a>], loc: Location) -> Result<TypedefSyntax<'a>, ParseError> {
        match sexpr.get(0) {
            Some(SExpr::Word("enum", loc)) => {
                Ok(TypedefSyntax::Enum(EnumSyntax::parse(&sexpr[1..], *loc)?))
            }
            Some(SExpr::Word("flags", loc)) => {
                Ok(TypedefSyntax::Flags(FlagsSyntax::parse(&sexpr[1..], *loc)?))
            }
            Some(SExpr::Word("struct", loc)) => Ok(TypedefSyntax::Struct(StructSyntax::parse(
                &sexpr[1..],
                *loc,
            )?)),
            _ => Err(parse_err!(loc, "invalid typedef")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumSyntax<'a> {
    pub repr: BuiltinType,
    pub members: Vec<&'a str>,
}

impl<'a> EnumSyntax<'a> {
    pub fn parse(sexpr: &[SExpr<'a>], loc: Location) -> Result<EnumSyntax<'a>, ParseError> {
        let repr = match sexpr.get(0) {
            Some(e) => BuiltinType::parse(e)?,
            _ => Err(parse_err!(loc, "no enum repr"))?,
        };
        let members = match sexpr.get(1) {
            Some(SExpr::Vec(v, loc)) => v
                .iter()
                .map(|m| match m {
                    SExpr::Ident(i, _) => Ok(*i),
                    _ => Err(parse_err!(*loc, "expected enum member identifier")),
                })
                .collect::<Result<Vec<_>, ParseError>>()?,
            _ => Err(parse_err!(loc, "empty enum members"))?,
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
    pub fn parse(sexpr: &[SExpr<'a>], loc: Location) -> Result<FlagsSyntax<'a>, ParseError> {
        let repr = BuiltinType::parse(
            sexpr
                .get(0)
                .ok_or_else(|| parse_err!(loc, "expected flag repr type"))?,
        )?;
        let flags = sexpr[1..]
            .iter()
            .map(|f| match f {
                SExpr::Ident(i, _) => Ok(*i),
                _ => Err(parse_err!(loc, "expected flag identifier")),
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
    pub fn parse(sexpr: &[SExpr<'a>], loc: Location) -> Result<StructSyntax<'a>, ParseError> {
        if sexpr.is_empty() {
            Err(parse_err!(loc, "empty struct"))?;
        }
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
    pub fn parse(sexpr: &SExpr<'a>) -> Result<StructFieldSyntax<'a>, ParseError> {
        match sexpr {
            SExpr::Vec(v, loc) => match v.get(0) {
                Some(SExpr::Word("field", _)) => {
                    let name = match v.get(1) {
                        Some(SExpr::Ident(i, _)) => i,
                        _ => Err(parse_err!(*loc, "expected struct name identifier"))?,
                    };
                    let type_ = TypeIdent::parse(
                        v.get(2)
                            .ok_or_else(|| parse_err!(*loc, "expected struct type identifier"))?,
                    )?;
                    Ok(StructFieldSyntax { name, type_ })
                }
                _ => Err(parse_err!(*loc, "expected struct field")),
            },
            _ => Err(parse_err!(sexpr.location(), "expected struct field vector")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuletypeSyntax<'a> {
    pub imports: Vec<ModuleImportSyntax<'a>>,
    pub funcs: Vec<ModuleFuncSyntax<'a>>,
}

impl<'a> ModuletypeSyntax<'a> {
    pub fn parse(sexprs: &[SExpr<'a>], loc: Location) -> Result<ModuletypeSyntax<'a>, ParseError> {
        Err(parse_unimp!(loc, "moduletype syntax"))
    }
}

#[derive(Debug, Clone)]
pub struct ModuleImportSyntax<'a> {
    pub name: &'a str,
    pub type_: ImportTypeSyntax<'a>,
}

impl<'a> ModuleImportSyntax<'a> {
    pub fn parse(sexpr: &SExpr<'a>) -> Result<ModuleImportSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub enum ImportTypeSyntax<'a> {
    Memory,
    Func(FunctionTypeSyntax<'a>),
}

impl<'a> ImportTypeSyntax<'a> {
    pub fn parse(sexpr: &SExpr<'a>) -> Result<ImportTypeSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionTypeSyntax<'a> {
    pub params: Vec<TypeIdent<'a>>,
    pub results: Vec<TypeIdent<'a>>,
}

impl<'a> FunctionTypeSyntax<'a> {
    pub fn parse(sexpr: &SExpr<'a>) -> Result<FunctionTypeSyntax<'a>, ParseError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct ParamSyntax<'a> {
    pub name: &'a str,
    pub type_: TypeIdent<'a>,
}

impl<'a> ParamSyntax<'a> {
    pub fn parse(sexpr: &SExpr<'a>) -> Result<ParamSyntax<'a>, ParseError> {
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
    pub fn parse(sexpr: &SExpr<'a>) -> Result<ModuleFuncSyntax<'a>, ParseError> {
        unimplemented!()
    }
}
