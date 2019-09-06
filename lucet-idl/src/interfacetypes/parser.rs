#![allow(unreachable_code)] // wip
#![allow(unused_variables)] // wip
#![allow(dead_code)] // wip
use super::sexpr::SExpr;
use crate::Location;

#[derive(Debug, Fail)]
#[fail(display = "{} at {:?}", _0, _1)]
pub struct ParseError {
    message: String,
    location: Location,
}

macro_rules! parse_err {
    ($loc:expr, $msg:expr) => {
        ParseError { message: $msg.to_string(), location: $loc }
    };
    ($loc:expr, $fmt:expr, $( $arg:expr ),+ ) => {
        ParseError { message: format!($fmt, $( $arg ),+), location: $loc }
    };
}

#[derive(Debug, Clone, Copy)]
pub enum BuiltinType {
    String,
    Data,
    U8,
    U16,
    U32,
    U64,
    S8,
    S16,
    S32,
    S64,
}

impl BuiltinType {
    pub fn starts_parsing(sexpr: &SExpr) -> bool {
        match sexpr {
            SExpr::Word("string", _)
            | SExpr::Word("data", _)
            | SExpr::Word("u8", _)
            | SExpr::Word("u16", _)
            | SExpr::Word("u32", _)
            | SExpr::Word("u64", _)
            | SExpr::Word("s8", _)
            | SExpr::Word("s16", _)
            | SExpr::Word("s32", _)
            | SExpr::Word("s64", _) => true,
            _ => false,
        }
    }
    pub fn parse(sexpr: &SExpr) -> Result<Self, ParseError> {
        match sexpr {
            SExpr::Word("string", _loc) => Ok(BuiltinType::String),
            SExpr::Word("data", _loc) => Ok(BuiltinType::Data),
            SExpr::Word("u8", _loc) => Ok(BuiltinType::U8),
            SExpr::Word("u16", _loc) => Ok(BuiltinType::U16),
            SExpr::Word("u32", _loc) => Ok(BuiltinType::U32),
            SExpr::Word("u64", _loc) => Ok(BuiltinType::U64),
            SExpr::Word("s8", _loc) => Ok(BuiltinType::S8),
            SExpr::Word("s16", _loc) => Ok(BuiltinType::S16),
            SExpr::Word("s32", _loc) => Ok(BuiltinType::S32),
            SExpr::Word("s64", _loc) => Ok(BuiltinType::S64),
            _ => Err(parse_err!(sexpr.location(), "invalid builtin type")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TypeIdentSyntax {
    Builtin(BuiltinType),
    Array(Box<TypeIdentSyntax>),
    Ident(String),
}

impl TypeIdentSyntax {
    pub fn starts_parsing(sexpr: &SExpr) -> bool {
        BuiltinType::starts_parsing(sexpr)
            || match sexpr {
                SExpr::Ident(_, _) => true,
                SExpr::Vec(v, _) => match (v.get(0), v.get(1)) {
                    (Some(SExpr::Word("array", _)), Some(_)) => true,
                    _ => false,
                },
                _ => false,
            }
    }
    pub fn parse(sexpr: &SExpr) -> Result<TypeIdentSyntax, ParseError> {
        if BuiltinType::starts_parsing(sexpr) {
            let builtin = BuiltinType::parse(sexpr)?;
            Ok(TypeIdentSyntax::Builtin(builtin))
        } else {
            match sexpr {
                SExpr::Ident(i, _loc) => Ok(TypeIdentSyntax::Ident(i.to_string())),
                SExpr::Vec(v, loc) => match (v.get(0), v.get(1)) {
                    (Some(SExpr::Word("array", _loc)), Some(expr)) => Ok(TypeIdentSyntax::Array(
                        Box::new(TypeIdentSyntax::parse(expr)?),
                    )),
                    _ => Err(parse_err!(*loc, "expected type identifier")),
                },
                _ => Err(parse_err!(sexpr.location(), "expected type identifier")),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TopLevelSyntax {
    Decl(DeclSyntax),
    Use(String),
}

impl TopLevelSyntax {
    pub fn parse(sexpr: &SExpr) -> Result<TopLevelSyntax, ParseError> {
        if DeclSyntax::starts_parsing(sexpr) {
            let decl = DeclSyntax::parse(sexpr)?;
            Ok(TopLevelSyntax::Decl(decl))
        } else {
            match sexpr {
                SExpr::Vec(v, loc) => match v.get(0) {
                    Some(SExpr::Word("use", loc)) => match v.get(1) {
                        Some(SExpr::Quote(u, _)) => Ok(TopLevelSyntax::Use(u.to_string())),
                        _ => Err(parse_err!(*loc, "invalid use declaration")),
                    },
                    _ => Err(parse_err!(
                        sexpr.location(),
                        "expected top level declaration"
                    )),
                },
                _ => Err(parse_err!(
                    sexpr.location(),
                    "expected top level declaration"
                )),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeclSyntax {
    Typename(TypenameSyntax),
    Module(ModuleSyntax),
}

impl DeclSyntax {
    pub fn starts_parsing(sexpr: &SExpr) -> bool {
        match sexpr {
            SExpr::Vec(v, _) => match v.get(0) {
                Some(SExpr::Word("typename", _)) => true,
                Some(SExpr::Word("module", _)) => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn parse(sexpr: &SExpr) -> Result<DeclSyntax, ParseError> {
        match sexpr {
            SExpr::Vec(v, loc) => match v.get(0) {
                Some(SExpr::Word("typename", loc)) => {
                    Ok(DeclSyntax::Typename(TypenameSyntax::parse(&v[1..], *loc)?))
                }
                Some(SExpr::Word("module", loc)) => {
                    Ok(DeclSyntax::Module(ModuleSyntax::parse(&v[1..], *loc)?))
                }
                _ => Err(parse_err!(*loc, "invalid declaration")),
            },
            _ => Err(parse_err!(sexpr.location(), "expected vec")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypenameSyntax {
    pub ident: String,
    pub def: TypedefSyntax,
}

impl TypenameSyntax {
    pub fn parse(sexpr: &[SExpr], loc: Location) -> Result<TypenameSyntax, ParseError> {
        let ident = match sexpr.get(0) {
            Some(SExpr::Ident(i, loc)) => i.to_string(),
            _ => Err(parse_err!(loc, "expected typename identifier"))?,
        };
        let def = match sexpr.get(1) {
            Some(expr) => TypedefSyntax::parse(expr)?,
            _ => Err(parse_err!(loc, "expected type definition"))?,
        };
        Ok(TypenameSyntax { ident, def })
    }
}

#[derive(Debug, Clone)]
pub enum TypedefSyntax {
    Ident(TypeIdentSyntax),
    Enum(EnumSyntax),
    Flags(FlagsSyntax),
    Struct(StructSyntax),
    Union(UnionSyntax),
}

impl TypedefSyntax {
    pub fn parse(sexpr: &SExpr) -> Result<TypedefSyntax, ParseError> {
        if TypeIdentSyntax::starts_parsing(sexpr) {
            let ident = TypeIdentSyntax::parse(sexpr)?;
            Ok(TypedefSyntax::Ident(ident))
        } else {
            match sexpr {
                SExpr::Vec(vs, loc) => match vs.get(0) {
                    Some(SExpr::Word("enum", loc)) => {
                        Ok(TypedefSyntax::Enum(EnumSyntax::parse(&vs[1..], *loc)?))
                    }
                    Some(SExpr::Word("flags", loc)) => {
                        Ok(TypedefSyntax::Flags(FlagsSyntax::parse(&vs[1..], *loc)?))
                    }
                    Some(SExpr::Word("struct", loc)) => {
                        Ok(TypedefSyntax::Struct(StructSyntax::parse(&vs[1..], *loc)?))
                    }
                    Some(SExpr::Word("union", loc)) => {
                        Ok(TypedefSyntax::Union(UnionSyntax::parse(&vs[1..], *loc)?))
                    }
                    _ => Err(parse_err!(
                        *loc,
                        "expected type identifier or type definition"
                    )),
                },
                _ => Err(parse_err!(
                    sexpr.location(),
                    "expected type identifier or type definition"
                )),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumSyntax {
    pub repr: BuiltinType,
    pub members: Vec<String>,
}

impl EnumSyntax {
    pub fn parse(sexpr: &[SExpr], loc: Location) -> Result<EnumSyntax, ParseError> {
        let repr = match sexpr.get(0) {
            Some(e) => BuiltinType::parse(e)?,
            _ => Err(parse_err!(loc, "no enum repr"))?,
        };
        let members = sexpr[1..]
            .iter()
            .map(|m| match m {
                SExpr::Ident(i, _) => Ok(i.to_string()),
                s => Err(parse_err!(s.location(), "expected enum member identifier")),
            })
            .collect::<Result<Vec<String>, ParseError>>()?;
        if members.is_empty() {
            Err(parse_err!(loc, "expected at least one enum member"))?
        }
        Ok(EnumSyntax { repr, members })
    }
}

#[derive(Debug, Clone)]
pub struct FlagsSyntax {
    pub repr: BuiltinType,
    pub flags: Vec<String>,
}

impl FlagsSyntax {
    pub fn parse(sexpr: &[SExpr], loc: Location) -> Result<FlagsSyntax, ParseError> {
        let repr = BuiltinType::parse(
            sexpr
                .get(0)
                .ok_or_else(|| parse_err!(loc, "expected flag repr type"))?,
        )?;
        let flags = sexpr[1..]
            .iter()
            .map(|f| match f {
                SExpr::Vec(vs, loc) => match (vs.get(0), vs.get(1)) {
                    (Some(SExpr::Word("flag", _)), Some(SExpr::Ident(i, _))) => Ok(i.to_string()),
                    _ => Err(parse_err!(*loc, "expected flag specifier")),
                },
                s => Err(parse_err!(s.location(), "expected flag specifier")),
            })
            .collect::<Result<Vec<_>, ParseError>>()?;
        Ok(FlagsSyntax { repr, flags })
    }
}

#[derive(Debug, Clone)]
pub struct StructSyntax {
    pub fields: Vec<FieldSyntax>,
}

impl StructSyntax {
    pub fn parse(sexpr: &[SExpr], loc: Location) -> Result<StructSyntax, ParseError> {
        if sexpr.is_empty() {
            Err(parse_err!(loc, "expected at least one struct member"))?
        }
        let fields = sexpr
            .iter()
            .map(|f| FieldSyntax::parse(f, "field"))
            .collect::<Result<Vec<_>, ParseError>>()?;
        Ok(StructSyntax { fields })
    }
}

#[derive(Debug, Clone)]
pub struct FieldSyntax {
    pub name: String,
    pub type_: TypeIdentSyntax,
}

impl FieldSyntax {
    pub fn starts_parsing(sexpr: &SExpr, constructor: &str) -> bool {
        match sexpr {
            SExpr::Vec(v, _) => match v.get(0) {
                Some(SExpr::Word(c, _)) => *c == constructor,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn parse(sexpr: &SExpr, constructor: &str) -> Result<FieldSyntax, ParseError> {
        match sexpr {
            SExpr::Vec(v, loc) => match v.get(0) {
                Some(SExpr::Word(c, _)) if *c == constructor => {
                    let name = match v.get(1) {
                        Some(SExpr::Ident(i, _)) => i.to_string(),
                        _ => Err(parse_err!(*loc, "expected {} name identifier", constructor))?,
                    };
                    let type_ = TypeIdentSyntax::parse(v.get(2).ok_or_else(|| {
                        parse_err!(*loc, "expected {} type identifier", constructor)
                    })?)?;
                    Ok(FieldSyntax { name, type_ })
                }
                _ => Err(parse_err!(*loc, "expected {}", constructor)),
            },
            _ => Err(parse_err!(sexpr.location(), "expected {}", constructor)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnionSyntax {
    pub fields: Vec<FieldSyntax>,
}

impl UnionSyntax {
    pub fn parse(sexpr: &[SExpr], loc: Location) -> Result<UnionSyntax, ParseError> {
        if sexpr.is_empty() {
            Err(parse_err!(loc, "expected at least one union member"))?
        }
        let fields = sexpr
            .iter()
            .map(|f| FieldSyntax::parse(f, "field"))
            .collect::<Result<Vec<_>, ParseError>>()?;
        Ok(UnionSyntax { fields })
    }
}

#[derive(Debug, Clone)]
pub struct ModuleSyntax {
    pub name: String,
    pub imports: Vec<ModuleImportSyntax>,
    pub funcs: Vec<InterfaceFuncSyntax>,
}

impl ModuleSyntax {
    pub fn parse(sexprs: &[SExpr], loc: Location) -> Result<ModuleSyntax, ParseError> {
        let name = match sexprs.get(0) {
            Some(SExpr::Ident(i, _)) => i.to_string(),
            _ => Err(parse_err!(loc, "expected module name"))?,
        };
        let mut imports = Vec::new();
        let mut funcs = Vec::new();
        for sexpr in &sexprs[1..] {
            if ModuleImportSyntax::starts_parsing(sexpr) {
                let import = ModuleImportSyntax::parse(sexpr)?;
                imports.push(import);
            } else if InterfaceFuncSyntax::starts_parsing(sexpr) {
                let func = InterfaceFuncSyntax::parse(sexpr)?;
                funcs.push(func);
            } else {
                Err(parse_err!(sexpr.location(), "expected import or function"))?;
            }
        }
        Ok(ModuleSyntax {
            name,
            imports,
            funcs,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ModuleImportSyntax {
    pub name: String,
    pub type_: ImportTypeSyntax,
}

impl ModuleImportSyntax {
    pub fn starts_parsing(sexpr: &SExpr) -> bool {
        match sexpr {
            SExpr::Vec(vs, _) => match vs.get(0) {
                Some(SExpr::Word("import", _)) => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn parse(sexpr: &SExpr) -> Result<ModuleImportSyntax, ParseError> {
        match sexpr {
            SExpr::Vec(vs, loc) => match (vs.get(0), vs.get(1)) {
                (Some(SExpr::Word("import", _)), Some(SExpr::Quote(name, _))) => {
                    let type_ = ImportTypeSyntax::parse(&vs[2..], *loc)?;
                    Ok(ModuleImportSyntax {
                        name: name.to_string(),
                        type_,
                    })
                }
                _ => Err(parse_err!(*loc, "expected module import")),
            },
            _ => Err(parse_err!(sexpr.location(), "expected module import")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ImportTypeSyntax {
    Memory,
}

impl ImportTypeSyntax {
    pub fn parse(sexpr: &[SExpr], loc: Location) -> Result<ImportTypeSyntax, ParseError> {
        if sexpr.len() > 1 {
            Err(parse_err!(loc, "too many elements for an import type"))?;
        }
        match sexpr.get(0) {
            Some(SExpr::Vec(vs, loc)) => match vs.get(0) {
                Some(SExpr::Word("memory", _)) => {
                    if vs.len() == 1 {
                        Ok(ImportTypeSyntax::Memory)
                    } else {
                        Err(parse_err!(*loc, "too many elements for memory declaration"))
                    }
                }
                _ => Err(parse_err!(*loc, "expected import type")),
            },
            _ => Err(parse_err!(loc, "expected import type")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceFuncSyntax {
    pub export: String,
    pub params: Vec<FieldSyntax>,
    pub results: Vec<FieldSyntax>,
}

impl InterfaceFuncSyntax {
    pub fn starts_parsing(sexpr: &SExpr) -> bool {
        match sexpr {
            SExpr::Vec(vs, _) => match (vs.get(0), vs.get(1)) {
                (Some(SExpr::Annot("interface", _)), Some(SExpr::Word("func", _))) => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn parse(sexpr: &SExpr) -> Result<InterfaceFuncSyntax, ParseError> {
        match sexpr {
            SExpr::Vec(vs, loc) => match (vs.get(0), vs.get(1)) {
                (Some(SExpr::Annot("interface", _)), Some(SExpr::Word("func", _))) => {
                    let export = match vs.get(2) {
                        Some(SExpr::Vec(es, loc)) => match (es.get(0), es.get(1)) {
                            (Some(SExpr::Word("export", _)), Some(SExpr::Quote(name, _))) => {
                                if es.len() == 2 {
                                    name.to_string()
                                } else {
                                    Err(parse_err!(
                                        *loc,
                                        "too many elements for export declaration"
                                    ))?
                                }
                            }
                            _ => Err(parse_err!(*loc, "expected export declaration"))?,
                        },
                        _ => Err(parse_err!(*loc, "expected export declaration"))?,
                    };
                    let mut params = Vec::new();
                    let mut results = Vec::new();

                    for sexpr in &vs[3..] {
                        if FieldSyntax::starts_parsing(sexpr, "param") {
                            let param = FieldSyntax::parse(sexpr, "param")?;
                            params.push(param);
                        } else if FieldSyntax::starts_parsing(sexpr, "result") {
                            let result = FieldSyntax::parse(sexpr, "result")?;
                            results.push(result);
                        } else {
                            Err(parse_err!(
                                sexpr.location(),
                                "expected param or result field"
                            ))?;
                        }
                    }

                    Ok(InterfaceFuncSyntax {
                        export,
                        params,
                        results,
                    })
                }
                _ => Err(parse_err!(*loc, "expected interface func declaration")),
            },

            _ => Err(parse_err!(
                sexpr.location(),
                "expected interface func declaration"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParamSyntax {
    pub name: String,
    pub type_: TypeIdentSyntax,
}

impl ParamSyntax {
    pub fn parse(sexpr: &SExpr) -> Result<ParamSyntax, ParseError> {
        unimplemented!()
    }
}
