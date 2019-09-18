use super::lexer::{LexError, Lexer, LocatedError, LocatedToken, Token};
use super::Location;
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PackageDecl<'a> {
    Module {
        name: &'a str,
        decls: Vec<ModuleDecl<'a>>,
        location: Location,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ModuleDecl<'a> {
    Struct {
        name: &'a str,
        members: Vec<StructMember<'a>>,
        location: Location,
    },
    Enum {
        name: &'a str,
        variants: Vec<EnumVariant<'a>>,
        location: Location,
    },
    Alias {
        name: &'a str,
        what: SyntaxIdent<'a>,
        location: Location,
    },
    Function {
        name: &'a str,
        args: Vec<FuncArgSyntax<'a>>,
        rets: Vec<FuncArgSyntax<'a>>,
        bindings: Vec<BindingSyntax<'a>>,
        location: Location,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SyntaxIdent<'a> {
    pub name: &'a str,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMember<'a> {
    pub name: &'a str,
    pub type_: SyntaxIdent<'a>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumVariant<'a> {
    pub name: &'a str,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncArgSyntax<'a> {
    pub name: &'a str,
    pub type_: SyntaxIdent<'a>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseError {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BindingDirSyntax {
    In,
    Out,
    InOut,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BindingSyntax<'a> {
    pub name: &'a str,
    pub type_: SyntaxIdent<'a>,
    pub direction: BindingDirSyntax,
    pub from: BindingRefSyntax<'a>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BindingRefSyntax<'a> {
    Ptr(Box<BindingRefSyntax<'a>>),
    Slice(Box<BindingRefSyntax<'a>>, Box<BindingRefSyntax<'a>>),
    Name(&'a str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at line {} column {}: {}",
            self.location.line, self.location.column, self.message
        )
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        "Parse error"
    }
}

macro_rules! parse_err {
    ($loc:expr, $msg: expr ) => {
        Err(ParseError {
            location: $loc.clone(),
            message: $msg.to_string(),
        })
    };

    ($loc:expr, $fmt:expr, $( $arg:expr),+ ) => {
        Err(ParseError {
            location: $loc.clone(),
            message: format!( $fmt, $( $arg ),+ ),
        })
    };
}
macro_rules! err_ctx {
    ($ctx:expr, $res:expr) => {
        match $res {
            Ok(a) => Ok(a),
            Err(ParseError { location, message }) => Err(ParseError {
                location,
                message: format!("in {}:\n{}", $ctx, message),
            }),
        }
    };
}

pub struct Parser<'a> {
    lex: Lexer<'a>,
    lookahead: Option<Token<'a>>,
    pub lex_error: Option<LexError>,
    location: Location,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Parser<'_> {
        Parser {
            lex: Lexer::new(text),
            lookahead: None,
            lex_error: None,
            location: Location { line: 0, column: 0 },
        }
    }
    fn consume(&mut self) -> Token<'a> {
        self.lookahead.take().expect("no token to consume")
    }
    fn token(&mut self) -> Option<Token<'a>> {
        while self.lookahead == None {
            match self.lex.next() {
                Some(Ok(LocatedToken { token, location })) => {
                    self.location = location;
                    self.lookahead = Some(token)
                }
                Some(Err(LocatedError { error, location })) => {
                    self.location = location;
                    self.lex_error = Some(error);
                    break;
                }
                None => break,
            }
        }
        self.lookahead
    }

    fn match_token(&mut self, want: Token<'a>, err_msg: &str) -> Result<Token<'a>, ParseError> {
        if self.token() == Some(want) {
            Ok(self.consume())
        } else {
            parse_err!(self.location, err_msg)
        }
    }

    fn match_a_word(&mut self, err_msg: &str) -> Result<&'a str, ParseError> {
        match self.token() {
            Some(Token::Word(text)) => {
                self.consume();
                Ok(text)
            }
            t => parse_err!(self.location, "{}, got {:?}", err_msg, t),
        }
    }

    fn match_ident(&mut self, err_msg: &str) -> Result<SyntaxIdent<'a>, ParseError> {
        match self.token() {
            Some(Token::Word(name)) => {
                let location = self.location;
                self.consume();
                Ok(SyntaxIdent { name, location })
            }
            _ => err_ctx!(err_msg, parse_err!(self.location, "expected identifier")),
        }
    }

    fn match_struct_body(&mut self) -> Result<Vec<StructMember<'a>>, ParseError> {
        let mut members = Vec::new();
        loop {
            match self.token() {
                Some(Token::RBrace) => {
                    self.consume();
                    break;
                }
                Some(Token::Word(member_name)) => {
                    let location = self.location;
                    self.consume();
                    self.match_token(Token::Colon, "expected :")?;
                    let member_ref = self.match_ident("expected member type")?;
                    members.push(StructMember {
                        name: member_name,
                        type_: member_ref,
                        location,
                    });
                    match self.token() {
                        Some(Token::Comma) => {
                            self.consume();
                            continue;
                        }
                        Some(Token::RBrace) => {
                            self.consume();
                            break;
                        }
                        _ => parse_err!(self.location, "in struct body:\nexpected , or '}'")?,
                    }
                }
                _ => parse_err!(
                    self.location,
                    "in struct body:\nexpected member name or '}'"
                )?,
            }
        }
        Ok(members)
    }

    fn match_enum_body(&mut self) -> Result<Vec<EnumVariant<'a>>, ParseError> {
        let mut names = Vec::new();
        loop {
            match self.token() {
                Some(Token::RBrace) => {
                    self.consume();
                    break;
                }
                Some(Token::Word(name)) => {
                    let location = self.location;
                    self.consume();
                    names.push(EnumVariant {
                        name: name,
                        location,
                    });
                    match self.token() {
                        Some(Token::Comma) => {
                            self.consume();
                            continue;
                        }
                        Some(Token::RBrace) => {
                            self.consume();
                            break;
                        }
                        _ => parse_err!(self.location, "expected , or }}")?,
                    }
                }
                _ => parse_err!(self.location, "expected variant")?,
            }
        }
        Ok(names)
    }

    fn match_func_args(&mut self) -> Result<Vec<FuncArgSyntax<'a>>, ParseError> {
        let mut args = Vec::new();
        loop {
            match self.token() {
                Some(Token::RPar) => {
                    self.consume();
                    break;
                }
                Some(Token::Word(name)) => {
                    let location = self.location;
                    self.consume();
                    self.match_token(Token::Colon, "expected :")?;
                    let type_ = self.match_ident("type name")?;

                    args.push(FuncArgSyntax {
                        name,
                        type_,
                        location,
                    });
                    match self.token() {
                        Some(Token::Comma) => {
                            self.consume();
                            continue;
                        }
                        Some(Token::RPar) => {
                            self.consume();
                            break;
                        }
                        _ => parse_err!(self.location, "expected , or )")?,
                    }
                }
                _ => parse_err!(self.location, "expected argument, or )")?,
            }
        }
        Ok(args)
    }

    fn match_func_rets(&mut self) -> Result<Vec<FuncArgSyntax<'a>>, ParseError> {
        let mut args = Vec::new();
        loop {
            match self.token() {
                Some(Token::Semi) | Some(Token::Word("where")) => {
                    break;
                }
                Some(Token::Word(name)) => {
                    let location = self.location;
                    self.consume();
                    self.match_token(Token::Colon, "expected :")?;
                    let type_ = self.match_ident("type name")?;
                    args.push(FuncArgSyntax {
                        type_,
                        name,
                        location,
                    });
                    match self.token() {
                        Some(Token::Comma) => {
                            self.consume();
                            continue;
                        }
                        Some(Token::Semi) | Some(Token::Word("where")) => {
                            break;
                        }
                        x => parse_err!(self.location, "expected ',', where, or ;, got {:?}", x)?,
                    }
                }
                x => parse_err!(self.location, "expected func return value, got {:?}", x)?,
            }
        }
        Ok(args)
    }

    fn match_binding_exprs(&mut self) -> Result<Vec<BindingSyntax<'a>>, ParseError> {
        let mut bindings = Vec::new();
        loop {
            match self.token() {
                Some(Token::Semi) => {
                    self.consume();
                    break;
                }
                Some(Token::Word(name)) => {
                    let location = self.location;
                    self.consume();
                    self.match_token(Token::Colon, "expected :")?;
                    let direction = self.match_bind_direction()?;
                    let type_ = self.match_ident("type name")?;
                    self.match_token(Token::LArrow, "expected <-")?;
                    let from = self.match_binding_ref()?;
                    bindings.push(BindingSyntax {
                        name,
                        type_,
                        direction,
                        from,
                        location,
                    });
                    match self.token() {
                        Some(Token::Semi) => {
                            self.consume();
                            break;
                        }
                        Some(Token::Comma) => {
                            self.consume();
                            continue;
                        }
                        _ => parse_err!(self.location, "expected , or ;")?,
                    }
                }
                _ => parse_err!(self.location, "expected binding expression")?,
            }
        }
        Ok(bindings)
    }

    fn match_bind_direction(&mut self) -> Result<BindingDirSyntax, ParseError> {
        match self.token() {
            Some(Token::Word("in")) => {
                self.consume();
                Ok(BindingDirSyntax::In)
            }
            Some(Token::Word("inout")) => {
                self.consume();
                Ok(BindingDirSyntax::InOut)
            }
            Some(Token::Word("out")) => {
                self.consume();
                Ok(BindingDirSyntax::Out)
            }
            _ => parse_err!(self.location, "expected binding direction (in, out, inout)"),
        }
    }

    fn match_binding_ref(&mut self) -> Result<BindingRefSyntax<'a>, ParseError> {
        match self.token() {
            Some(Token::Star) => {
                self.consume();
                Ok(BindingRefSyntax::Ptr(Box::new(self.match_binding_ref()?)))
            }
            Some(Token::Word(name)) => {
                self.consume();
                Ok(BindingRefSyntax::Name(name))
            }
            Some(Token::LBracket) => {
                self.consume();
                let ptr_arg = self.match_binding_ref()?;
                let _ = self.match_token(Token::Comma, ", in binding ref slice");
                let len_arg = self.match_binding_ref()?;
                let _ = self.match_token(Token::RBracket, "] at end of binding ref slice");
                Ok(BindingRefSyntax::Slice(
                    Box::new(ptr_arg),
                    Box::new(len_arg),
                ))
            }
            x => parse_err!(self.location, "expected binding ref, got {:?}", x),
        }
    }

    pub fn match_module_decl(&mut self) -> Result<ModuleDecl<'a>, ParseError> {
        match self.token() {
            Some(Token::Word("struct")) => {
                let location = self.location;
                self.consume();
                let name = self.match_a_word("expected struct name")?;
                self.match_token(Token::LBrace, "expected {")?;
                let members = self.match_struct_body()?;
                Ok(ModuleDecl::Struct {
                    name,
                    members,
                    location,
                })
            }
            Some(Token::Word("enum")) => {
                let location = self.location;
                self.consume();
                let name = self.match_a_word("expected enum name")?;
                self.match_token(Token::LBrace, "expected {")?;
                let variants = self.match_enum_body()?;
                Ok(ModuleDecl::Enum {
                    name,
                    variants,
                    location,
                })
            }
            Some(Token::Word("type")) => {
                let location = self.location;
                self.consume();
                let name = self.match_a_word("expected type name")?;
                self.match_token(Token::Equals, "expected =")?;
                let what = self.match_ident("type value")?;
                self.match_token(Token::Semi, "expected ;")?;
                Ok(ModuleDecl::Alias {
                    name,
                    what,
                    location,
                })
            }
            Some(Token::Word("fn")) => {
                let location = self.location;
                self.consume();
                let name = self.match_a_word("expected function name")?;

                self.match_token(Token::LPar, "expected (")?;
                let args = self.match_func_args()?;
                let rets = if let Some(Token::RArrow) = self.token() {
                    self.consume();
                    self.match_func_rets()?
                } else {
                    Vec::new()
                };

                let bindings = match self.token() {
                    Some(Token::Semi) => {
                        self.consume();
                        Vec::new()
                    }
                    Some(Token::Word("where")) => {
                        self.consume();
                        self.match_binding_exprs()?
                    }
                    t => parse_err!(self.location, "expected where, -> or ;, got {:?}", t)?,
                };

                Ok(ModuleDecl::Function {
                    name,
                    args,
                    rets,
                    bindings,
                    location,
                })
            }
            Some(_) | None => parse_err!(self.location, "expected module declaration"),
        }
    }

    #[cfg(test)]
    pub fn match_module_decls(&mut self) -> Result<Vec<ModuleDecl<'a>>, ParseError> {
        let mut decls = Vec::new();
        while self.token().is_some() {
            decls.push(self.match_module_decl()?);
        }
        Ok(decls)
    }

    pub fn match_package_decl(&mut self) -> Result<PackageDecl<'a>, ParseError> {
        match self.token() {
            Some(Token::Word("mod")) => {
                let location = self.location;
                self.consume();
                let name = self.match_a_word("expected module name")?;
                self.match_token(Token::LBrace, "expected {")?;

                let mut decls = Vec::new();
                loop {
                    match self.token() {
                        Some(Token::RBrace) => {
                            self.consume();
                            break;
                        }
                        Some(_) => {
                            let decl = self.match_module_decl()?;
                            decls.push(decl);
                        }
                        None => parse_err!(self.location, "expected module decl or }")?,
                    }
                }

                Ok(PackageDecl::Module {
                    name,
                    decls,
                    location,
                })
            }
            Some(_) | None => parse_err!(self.location, "expected package declaration"),
        }
    }

    pub fn match_package_decls(&mut self) -> Result<Vec<PackageDecl<'a>>, ParseError> {
        let mut decls = Vec::new();
        loop {
            match self.token() {
                Some(_) => {
                    let decl = self.match_package_decl()?;
                    decls.push(decl);
                }
                None => break,
            }
        }
        Ok(decls)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn struct_empty() {
        let mut parser = Parser::new("struct foo {}");
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Struct {
                name: "foo",
                members: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        );
    }
    #[test]
    fn struct_one_int_member() {
        let mut parser = Parser::new("struct foo {a: i32 }");
        // column ruler:              0      7    12 15
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Struct {
                name: "foo",
                members: vec![StructMember {
                    name: "a",
                    type_: SyntaxIdent {
                        name: "i32",
                        location: Location {
                            line: 1,
                            column: 15,
                        },
                    },
                    location: Location {
                        line: 1,
                        column: 12,
                    },
                }],
                location: Location { line: 1, column: 0 },
            }
        );
    }
    #[test]
    fn struct_one_int_member_trailing_comma() {
        let mut parser = Parser::new("struct foo {b: i32, }");
        //                            0      7    12 15
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Struct {
                name: "foo",
                members: vec![StructMember {
                    name: "b",
                    type_: SyntaxIdent {
                        name: "i32",
                        location: Location {
                            line: 1,
                            column: 15,
                        },
                    },
                    location: Location {
                        line: 1,
                        column: 12,
                    },
                }],
                location: Location { line: 1, column: 0 },
            }
        );
    }
    #[test]
    fn struct_two_int_members() {
        let mut parser = Parser::new("struct c { d: f64, e: u8 }");
        //                            0      7   11 14   19 22
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Struct {
                name: "c",
                members: vec![
                    StructMember {
                        name: "d",
                        type_: SyntaxIdent {
                            name: "f64",
                            location: Location {
                                line: 1,
                                column: 14,
                            },
                        },
                        location: Location {
                            line: 1,
                            column: 11,
                        },
                    },
                    StructMember {
                        name: "e",
                        type_: SyntaxIdent {
                            name: "u8",
                            location: Location {
                                line: 1,
                                column: 22,
                            },
                        },
                        location: Location {
                            line: 1,
                            column: 19,
                        },
                    },
                ],
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn struct_reserved_members() {
        let mut parser = Parser::new("struct foo {a: mod, struct: enum }");
        // column ruler:              0      7    12 15   21      30
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Struct {
                name: "foo",
                members: vec![
                    StructMember {
                        name: "a",
                        type_: SyntaxIdent {
                            name: "mod",
                            location: Location {
                                line: 1,
                                column: 15,
                            },
                        },
                        location: Location {
                            line: 1,
                            column: 12,
                        },
                    },
                    StructMember {
                        name: "struct",
                        type_: SyntaxIdent {
                            name: "enum",
                            location: Location {
                                line: 1,
                                column: 28,
                            },
                        },
                        location: Location {
                            line: 1,
                            column: 20,
                        },
                    }
                ],
                location: Location { line: 1, column: 0 },
            }
        );
    }
    #[test]
    fn enum_empty() {
        let mut parser = Parser::new("enum foo {}");
        //                            0    5
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Enum {
                name: "foo",
                variants: Vec::new(),
                location: Location { line: 1, column: 0 },
            },
        );
    }
    #[test]
    fn enum_one_entry_trailing_comma() {
        let mut parser = Parser::new("enum foo {first,}");
        //                            0    5    10
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Enum {
                name: "foo",
                variants: vec![EnumVariant {
                    name: "first",
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                }],
                location: Location { line: 1, column: 0 },
            },
        );
    }
    #[test]
    fn enum_one_entry() {
        let mut parser = Parser::new("enum bar {first}");
        //                            0    5    10
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Enum {
                name: "bar",
                variants: vec![EnumVariant {
                    name: "first",
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                }],
                location: Location { line: 1, column: 0 },
            },
        );
    }

    #[test]
    fn enum_four_entry() {
        let mut parser = Parser::new("enum baz { one, two, three\n, four, }");
        //                            0    5     11   16   21     0 2
        assert_eq!(
            parser.match_module_decl().expect("valid parse"),
            ModuleDecl::Enum {
                name: "baz",
                variants: vec![
                    EnumVariant {
                        name: "one",
                        location: Location {
                            line: 1,
                            column: 11,
                        },
                    },
                    EnumVariant {
                        name: "two",
                        location: Location {
                            line: 1,
                            column: 16,
                        },
                    },
                    EnumVariant {
                        name: "three",
                        location: Location {
                            line: 1,
                            column: 21,
                        },
                    },
                    EnumVariant {
                        name: "four",
                        location: Location { line: 2, column: 2 },
                    },
                ],
                location: Location { line: 1, column: 0 },
            },
        );
    }

    #[test]
    fn mod_empty() {
        let mut parser = Parser::new("mod empty {}");
        //                            0    5    10
        assert_eq!(
            parser.match_package_decl().expect("valid parse"),
            PackageDecl::Module {
                name: "empty",
                decls: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn mod_types() {
        let mut parser = Parser::new("mod one { enum foo {} struct bar {} }");
        //                            0    5    10   15   20
        assert_eq!(
            parser.match_package_decl().expect("valid parse"),
            PackageDecl::Module {
                name: "one",
                decls: vec![
                    ModuleDecl::Enum {
                        name: "foo",
                        variants: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 10
                        },
                    },
                    ModuleDecl::Struct {
                        name: "bar",
                        members: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 22
                        },
                    }
                ],
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn fn_trivial() {
        let canonical = ModuleDecl::Function {
            name: "trivial",
            args: Vec::new(),
            rets: Vec::new(),
            bindings: Vec::new(),
            location: Location { line: 1, column: 0 },
        };
        assert_eq!(
            Parser::new("fn trivial();")
                //               0    5    10
                .match_module_decl()
                .expect("valid parse"),
            canonical,
        );
        assert_eq!(
            Parser::new("fn trivial ( ) ;")
                //               0    5    10
                .match_module_decl()
                .expect("valid parse"),
            canonical,
        );
        assert_eq!(
            Parser::new("fn trivial()->;")
                //               0    5    10
                .match_module_decl()
                .expect("valid parse"),
            canonical,
        );
    }

    #[test]
    fn fn_return_i32() {
        fn canonical(column: usize) -> ModuleDecl<'static> {
            ModuleDecl::Function {
                name: "getch",
                args: Vec::new(),
                rets: vec![FuncArgSyntax {
                    type_: SyntaxIdent {
                        name: "i32",
                        location: Location {
                            line: 1,
                            column: column,
                        },
                    },
                    name: "r",
                    location: Location {
                        line: 1,
                        column: 14,
                    },
                }],
                bindings: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        }
        assert_eq!(
            Parser::new("fn getch() -> r:i32;")
                //       0    5    10   15
                .match_module_decl()
                .expect("valid decls"),
            canonical(16)
        );
        assert_eq!(
            Parser::new("fn getch() -> r: i32,;")
                //       0    5    10
                .match_module_decl()
                .expect("valid decls"),
            canonical(17)
        );
        assert_eq!(
            Parser::new("fn getch() -> r :i32 , ;")
                //       0    5    10
                .match_module_decl()
                .expect("valid decls"),
            canonical(17)
        );
    }

    #[test]
    fn fn_one_arg() {
        let canonical = ModuleDecl::Function {
            name: "foo",
            args: vec![FuncArgSyntax {
                type_: SyntaxIdent {
                    name: "i32",
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                },
                name: "a",
                location: Location { line: 1, column: 7 },
            }],
            rets: Vec::new(),
            bindings: Vec::new(),
            location: Location { line: 1, column: 0 },
        };
        assert_eq!(
            Parser::new("fn foo(a: i32);")
                //       0    5    10   15   20    25
                .match_module_decl()
                .expect("valid parse"),
            canonical
        );
        assert_eq!(
            Parser::new("fn foo(a: i32,);")
                //       0    5    10   15   20    25
                .match_module_decl()
                .expect("valid parse"),
            canonical
        );
    }

    #[test]
    fn fn_multi_arg() {
        let canonical = ModuleDecl::Function {
            name: "foo",
            args: vec![
                FuncArgSyntax {
                    type_: SyntaxIdent {
                        name: "i32",
                        location: Location {
                            line: 1,
                            column: 10,
                        },
                    },
                    name: "a",
                    location: Location { line: 1, column: 7 },
                },
                FuncArgSyntax {
                    type_: SyntaxIdent {
                        name: "f64",
                        location: Location {
                            line: 1,
                            column: 18,
                        },
                    },
                    name: "b",
                    location: Location {
                        line: 1,
                        column: 15,
                    },
                },
            ],
            rets: Vec::new(),
            bindings: Vec::new(),
            location: Location { line: 1, column: 0 },
        };
        assert_eq!(
            Parser::new("fn foo(a: i32, b: f64);")
                //       0    5    10   15   20    25
                .match_module_decl()
                .expect("valid parse"),
            canonical
        );
        assert_eq!(
            Parser::new("fn foo(a: i32, b: f64, );")
                //       0    5    10   15   20    25
                .match_module_decl()
                .expect("valid parse"),
            canonical
        );
    }

    #[test]
    fn fn_many_returns() {
        assert_eq!(
            Parser::new("fn getch() -> r1: i32, r2: i64, r3: f32;")
                //       0    5    10   15   20   25   30
                .match_module_decl()
                .expect("valid parse"),
            ModuleDecl::Function {
                name: "getch",
                args: Vec::new(),
                rets: vec![
                    FuncArgSyntax {
                        type_: SyntaxIdent {
                            name: "i32",
                            location: Location {
                                line: 1,
                                column: 18,
                            }
                        },
                        name: "r1",
                        location: Location {
                            line: 1,
                            column: 14,
                        },
                    },
                    FuncArgSyntax {
                        type_: SyntaxIdent {
                            name: "i64",
                            location: Location {
                                line: 1,
                                column: 27
                            }
                        },
                        name: "r2",
                        location: Location {
                            line: 1,
                            column: 23,
                        },
                    },
                    FuncArgSyntax {
                        type_: SyntaxIdent {
                            name: "f32",
                            location: Location {
                                line: 1,
                                column: 36
                            }
                        },
                        name: "r3",
                        location: Location {
                            line: 1,
                            column: 32,
                        },
                    },
                ],
                bindings: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn fn_with_bindings() {
        assert_eq!(
            Parser::new(
                "fn fgetch(fptr: i32) -> r: i32 where \n\
                 file: in file_t <- *fptr,\n\
                 r: out u8 <- r,\n\
                 some_slice: out something <- [a, b];"
            )
            //   0    5    10   15   20   25   30
            .match_module_decl()
            .expect("valid parse"),
            ModuleDecl::Function {
                name: "fgetch",
                args: vec![FuncArgSyntax {
                    type_: SyntaxIdent {
                        name: "i32",
                        location: Location {
                            line: 1,
                            column: 16
                        }
                    },
                    name: "fptr",
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                },],
                rets: vec![FuncArgSyntax {
                    type_: SyntaxIdent {
                        name: "i32",
                        location: Location {
                            line: 1,
                            column: 27
                        }
                    },
                    name: "r",
                    location: Location {
                        line: 1,
                        column: 24,
                    },
                }],
                bindings: vec![
                    BindingSyntax {
                        name: "file",
                        type_: SyntaxIdent {
                            name: "file_t",
                            location: Location { line: 2, column: 9 },
                        },
                        direction: BindingDirSyntax::In,
                        from: BindingRefSyntax::Ptr(Box::new(BindingRefSyntax::Name("fptr"))),
                        location: Location { line: 2, column: 0 },
                    },
                    BindingSyntax {
                        name: "r",
                        type_: SyntaxIdent {
                            name: "u8",
                            location: Location { line: 3, column: 7 },
                        },
                        direction: BindingDirSyntax::Out,
                        from: BindingRefSyntax::Name("r"),
                        location: Location { line: 3, column: 0 },
                    },
                    BindingSyntax {
                        name: "some_slice",
                        type_: SyntaxIdent {
                            name: "something",
                            location: Location {
                                line: 4,
                                column: 16
                            },
                        },
                        direction: BindingDirSyntax::Out,
                        from: BindingRefSyntax::Slice(
                            Box::new(BindingRefSyntax::Name("a")),
                            Box::new(BindingRefSyntax::Name("b"))
                        ),
                        location: Location { line: 4, column: 0 },
                    }
                ],
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn no_mod_in_mod() {
        let err = Parser::new("mod foo { mod bar { }}").match_package_decl();
        assert_eq!(
            err,
            Err(ParseError {
                message: "expected module declaration".to_owned(),
                location: Location {
                    line: 1,
                    column: 10
                }
            })
        );
        Parser::new("mod foo { enum whatever {} mod bar { }}")
            .match_package_decls()
            .err()
            .expect("error package");
    }

    #[test]
    fn no_top_level_types() {
        let err = Parser::new("mod foo { } enum bar {}")
            .match_package_decls()
            .err()
            .expect("error package");
        assert_eq!(
            err,
            ParseError {
                message: "expected package declaration".to_owned(),
                location: Location {
                    line: 1,
                    column: 12
                }
            }
        );
    }
}
