use super::lexer::{LexError, Lexer, LocatedError, LocatedToken, Token};
use super::types::{AbiType, AtomType, BindDirection, Location};
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyntaxDecl {
    Struct {
        name: String,
        members: Vec<StructMember>,
        location: Location,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        location: Location,
    },
    Alias {
        name: String,
        what: SyntaxRef,
        location: Location,
    },
    Module {
        name: String,
        decls: Vec<SyntaxDecl>,
        location: Location,
    },
    Function {
        name: String,
        args: Vec<FuncArgSyntax>,
        rets: Vec<FuncArgSyntax>,
        bindings: Vec<BindingSyntax>,
        location: Location,
    },
}

impl SyntaxDecl {
    pub fn name(&self) -> &str {
        match self {
            SyntaxDecl::Struct { name, .. } => &name,
            SyntaxDecl::Enum { name, .. } => &name,
            SyntaxDecl::Alias { name, .. } => &name,
            SyntaxDecl::Module { name, .. } => &name,
            SyntaxDecl::Function { name, .. } => &name,
        }
    }
    pub fn location(&self) -> &Location {
        match self {
            SyntaxDecl::Struct { location, .. } => &location,
            SyntaxDecl::Enum { location, .. } => &location,
            SyntaxDecl::Alias { location, .. } => &location,
            SyntaxDecl::Module { location, .. } => &location,
            SyntaxDecl::Function { location, .. } => &location,
        }
    }
    pub fn is_datatype(&self) -> bool {
        match self {
            SyntaxDecl::Struct { .. } | SyntaxDecl::Enum { .. } | SyntaxDecl::Alias { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyntaxRef {
    Atom { atom: AtomType, location: Location },
    Name { name: String, location: Location },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StructMember {
    pub name: String,
    pub type_: SyntaxRef,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FuncArgSyntax {
    pub name: String,
    pub type_: AbiType,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseError {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BindingSyntax {
    pub name: String,
    pub type_: SyntaxRef,
    pub direction: BindDirection,
    pub from: BindingRefSyntax,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BindingRefSyntax {
    Ptr(Box<BindingRefSyntax>),
    Slice(Box<BindingRefSyntax>, Box<BindingRefSyntax>),
    Name(String),
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

    fn match_struct_body(&mut self) -> Result<Vec<StructMember>, ParseError> {
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
                    let member_ref = self.match_ref("expected member type")?;
                    members.push(StructMember {
                        name: member_name.to_string(),
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

    fn match_enum_body(&mut self) -> Result<Vec<EnumVariant>, ParseError> {
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
                        name: name.to_owned(),
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

    fn match_func_args(&mut self) -> Result<Vec<FuncArgSyntax>, ParseError> {
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
                    let type_ = self.match_abitype()?;

                    args.push(FuncArgSyntax {
                        name: name.to_string(),
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

    fn match_func_rets(&mut self) -> Result<Vec<FuncArgSyntax>, ParseError> {
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
                    let type_ = self.match_abitype()?;
                    args.push(FuncArgSyntax {
                        type_,
                        name: name.to_string(),
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

    fn match_binding_exprs(&mut self) -> Result<Vec<BindingSyntax>, ParseError> {
        let mut bindings = Vec::new();
        loop {
            match self.token() {
                Some(Token::Semi) => break,
                Some(Token::Word(name)) => {
                    let location = self.location;
                    self.consume();
                    self.match_token(Token::Colon, "expected :")?;
                    let type_ = self.match_ref("type value")?;
                    self.match_token(Token::LArrow, "expected <-")?;
                    let direction = self.match_bind_direction()?;
                    let from = self.match_binding_ref()?;
                    bindings.push(BindingSyntax {
                        name: name.to_string(),
                        type_,
                        direction,
                        from,
                        location,
                    });
                    match self.token() {
                        Some(Token::Semi) => break,
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

    fn match_bind_direction(&mut self) -> Result<BindDirection, ParseError> {
        match self.token() {
            Some(Token::Word("in")) => {
                self.consume();
                Ok(BindDirection::In)
            }
            Some(Token::Word("inout")) | Some(Token::Word("io")) => {
                self.consume();
                Ok(BindDirection::InOut)
            }
            Some(Token::Word("out")) => {
                self.consume();
                Ok(BindDirection::Out)
            }
            _ => parse_err!(
                self.location,
                "expected binding direction (in, out, inout, io)"
            ),
        }
    }

    fn match_binding_ref(&mut self) -> Result<BindingRefSyntax, ParseError> {
        match self.token() {
            Some(Token::Star) => {
                self.consume();
                Ok(BindingRefSyntax::Ptr(Box::new(self.match_binding_ref()?)))
            }
            Some(Token::Word(name)) => {
                self.consume();
                Ok(BindingRefSyntax::Name(name.to_string()))
            }
            x => parse_err!(self.location, "expected binding ref, got {:?}", x),
        }
    }

    pub fn match_decl(&mut self, err_msg: &str) -> Result<Option<SyntaxDecl>, ParseError> {
        loop {
            match self.token() {
                Some(Token::Word("struct")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected struct name"))?;
                    err_ctx!(err_msg, self.match_token(Token::LBrace, "expected {"))?;
                    let members = err_ctx!(err_msg, self.match_struct_body())?;
                    return Ok(Some(SyntaxDecl::Struct {
                        name: name.to_owned(),
                        members,
                        location,
                    }));
                }
                Some(Token::Word("enum")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected enum name"))?;
                    err_ctx!(err_msg, self.match_token(Token::LBrace, "expected {"))?;
                    let variants = err_ctx!(err_msg, self.match_enum_body())?;
                    return Ok(Some(SyntaxDecl::Enum {
                        name: name.to_owned(),
                        variants,
                        location,
                    }));
                }
                Some(Token::Word("type")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected type name"))?;
                    err_ctx!(err_msg, self.match_token(Token::Equals, "expected ="))?;
                    let what = self.match_ref("type value")?;
                    err_ctx!(err_msg, self.match_token(Token::Semi, "expected ;"))?;
                    return Ok(Some(SyntaxDecl::Alias {
                        name: name.to_owned(),
                        what,
                        location,
                    }));
                }
                Some(Token::Word("mod")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected module name"))?;
                    err_ctx!(err_msg, self.match_token(Token::LBrace, "expected {"))?;

                    let mut decls = Vec::new();
                    loop {
                        if let Some(Token::RBrace) = self.token() {
                            self.consume();
                            break;
                        } else {
                            match self.match_decl("declaration") {
                                Ok(Some(decl)) => decls.push(decl),
                                Ok(None) => parse_err!(self.location, "missing close brace '}'")?,
                                Err(e) => Err(e)?,
                            }
                        }
                    }

                    return Ok(Some(SyntaxDecl::Module {
                        name: name.to_owned(),
                        decls,
                        location,
                    }));
                }
                Some(Token::Word("fn")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected function name"))?;

                    err_ctx!(err_msg, self.match_token(Token::LPar, "expected ("))?;
                    let args = err_ctx!(err_msg, self.match_func_args())?;

                    let rets = match self.token() {
                        Some(Token::RArrow) => {
                            self.consume();
                            err_ctx!(err_msg, self.match_func_rets())?
                        }
                        Some(Token::Semi) | Some(Token::Word("where")) => Vec::new(),
                        t => err_ctx!(
                            err_msg,
                            parse_err!(self.location, "expected where, -> or ;, got {:?}", t)
                        )?,
                    };

                    let bindings = match self.token() {
                        Some(Token::Semi) => {
                            self.consume();
                            Vec::new()
                        }
                        Some(Token::Word("where")) => {
                            self.consume();
                            err_ctx!(err_msg, self.match_binding_exprs())?
                        }
                        x => unreachable!(
                            "match func rets didnt leave us with semi or where: {:?}",
                            x
                        ),
                    };

                    return Ok(Some(SyntaxDecl::Function {
                        name: name.to_owned(),
                        args,
                        rets,
                        bindings,
                        location,
                    }));
                }
                Some(_) => {
                    return parse_err!(
                        self.location,
                        "in {}\nexpected keyword or attribute",
                        err_msg
                    )
                }
                None => {
                    return Ok(None);
                }
            }
        }
    }

    pub fn match_decls(&mut self) -> Result<Vec<SyntaxDecl>, ParseError> {
        let mut decls = Vec::new();
        loop {
            match self.match_decl("declaration") {
                Ok(Some(decl)) => decls.push(decl),
                Ok(None) => break,
                Err(e) => Err(e)?,
            }
        }
        Ok(decls)
    }

    fn match_ref(&mut self, err_msg: &str) -> Result<SyntaxRef, ParseError> {
        match self.token() {
            Some(Token::Atom(atom)) => {
                let location = self.location;
                self.consume();
                Ok(SyntaxRef::Atom { atom, location })
            }
            Some(Token::Word(name)) => {
                let location = self.location;
                self.consume();
                Ok(SyntaxRef::Name {
                    name: name.to_string(),
                    location,
                })
            }
            _ => err_ctx!(
                err_msg,
                parse_err!(self.location, "expected atom, or type name")
            ),
        }
    }

    fn match_abitype(&mut self) -> Result<AbiType, ParseError> {
        match self.token() {
            Some(Token::Atom(atom)) => match AbiType::of_atom(atom) {
                Some(abitype) => {
                    self.consume();
                    Ok(abitype)
                }
                None => parse_err!(self.location, "expected abi type, got non-abi atom type"),
            },
            Some(Token::Word(w)) => parse_err!(self.location, "expected abi type, got '{}'", w),
            _ => parse_err!(self.location, "expected abi type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn struct_empty() {
        let mut parser = Parser::new("struct foo {}");
        assert_eq!(
            parser
                .match_decl("empty struct")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
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
            parser
                .match_decl("foo a i32")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: vec![StructMember {
                    name: "a".to_owned(),
                    type_: SyntaxRef::Atom {
                        atom: AtomType::I32,
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
            parser
                .match_decl("foo b i32 with trailing comma")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: vec![StructMember {
                    name: "b".to_owned(),
                    type_: SyntaxRef::Atom {
                        atom: AtomType::I32,
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
            parser
                .match_decl("struct c")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "c".to_string(),
                members: vec![
                    StructMember {
                        name: "d".to_owned(),
                        type_: SyntaxRef::Atom {
                            atom: AtomType::F64,
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
                        name: "e".to_owned(),
                        type_: SyntaxRef::Atom {
                            atom: AtomType::U8,
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
            parser
                .match_decl("foo a i32")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: vec![
                    StructMember {
                        name: "a".to_owned(),
                        type_: SyntaxRef::Name {
                            name: "mod".to_owned(),
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
                        name: "struct".to_owned(),
                        type_: SyntaxRef::Name {
                            name: "enum".to_owned(),
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
            parser
                .match_decl("empty enum")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Enum {
                name: "foo".to_owned(),
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
            parser
                .match_decl("one entry enum, trailing comma")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Enum {
                name: "foo".to_owned(),
                variants: vec![EnumVariant {
                    name: "first".to_owned(),
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
            parser
                .match_decl("one entry enum, no trailing comma")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Enum {
                name: "bar".to_owned(),
                variants: vec![EnumVariant {
                    name: "first".to_owned(),
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
            parser
                .match_decl("four entry enum, trailing comma")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Enum {
                name: "baz".to_owned(),
                variants: vec![
                    EnumVariant {
                        name: "one".to_owned(),
                        location: Location {
                            line: 1,
                            column: 11,
                        },
                    },
                    EnumVariant {
                        name: "two".to_owned(),
                        location: Location {
                            line: 1,
                            column: 16,
                        },
                    },
                    EnumVariant {
                        name: "three".to_owned(),
                        location: Location {
                            line: 1,
                            column: 21,
                        },
                    },
                    EnumVariant {
                        name: "four".to_owned(),
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
            parser
                .match_decl("empty module")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Module {
                name: "empty".to_owned(),
                decls: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn mod_nesting() {
        let mut parser = Parser::new("mod one { mod two { mod three { } } }");
        //                            0    5    10   15   20
        assert_eq!(
            parser
                .match_decl("nested modules")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Module {
                name: "one".to_owned(),
                decls: vec![SyntaxDecl::Module {
                    name: "two".to_owned(),
                    decls: vec![SyntaxDecl::Module {
                        name: "three".to_owned(),
                        decls: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 20
                        },
                    }],
                    location: Location {
                        line: 1,
                        column: 10
                    },
                }],
                location: Location { line: 1, column: 0 },
            }
        );
    }

    #[test]
    fn mod_types() {
        let mut parser = Parser::new("mod one { enum foo {} struct bar {} }");
        //                            0    5    10   15   20
        assert_eq!(
            parser
                .match_decl("module with types")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Module {
                name: "one".to_owned(),
                decls: vec![
                    SyntaxDecl::Enum {
                        name: "foo".to_owned(),
                        variants: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 10
                        },
                    },
                    SyntaxDecl::Struct {
                        name: "bar".to_owned(),
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
        let canonical = vec![SyntaxDecl::Function {
            name: "trivial".to_owned(),
            args: Vec::new(),
            rets: Vec::new(),
            bindings: Vec::new(),
            location: Location { line: 1, column: 0 },
        }];
        assert_eq!(
            Parser::new("fn trivial();")
                //               0    5    10
                .match_decls()
                .expect("valid parse"),
            canonical,
        );
        assert_eq!(
            Parser::new("fn trivial ( ) ;")
                //               0    5    10
                .match_decls()
                .expect("valid parse"),
            canonical,
        );
        assert_eq!(
            Parser::new("fn trivial()->;")
                //               0    5    10
                .match_decls()
                .expect("valid parse"),
            canonical,
        );
    }

    #[test]
    fn fn_return_i32() {
        let canonical = vec![SyntaxDecl::Function {
            name: "getch".to_owned(),
            args: Vec::new(),
            rets: vec![FuncArgSyntax {
                type_: AbiType::I32,
                name: "r".to_owned(),
                location: Location {
                    line: 1,
                    column: 14,
                },
            }],
            bindings: Vec::new(),
            location: Location { line: 1, column: 0 },
        }];
        assert_eq!(
            Parser::new("fn getch() -> r:i32;")
                //       0    5    10
                .match_decls()
                .expect("valid decls"),
            canonical
        );
        assert_eq!(
            Parser::new("fn getch() -> r: i32,;")
                //       0    5    10
                .match_decls()
                .expect("valid decls"),
            canonical
        );
        assert_eq!(
            Parser::new("fn getch() -> r :i32 , ;")
                //       0    5    10
                .match_decls()
                .expect("valid decls"),
            canonical
        );
    }

    #[test]
    fn fn_one_arg() {
        let canonical = SyntaxDecl::Function {
            name: "foo".to_owned(),
            args: vec![FuncArgSyntax {
                type_: AbiType::I32,
                name: "a".to_owned(),
                location: Location { line: 1, column: 7 },
            }],
            rets: Vec::new(),
            bindings: Vec::new(),
            location: Location { line: 1, column: 0 },
        };
        assert_eq!(
            Parser::new("fn foo(a: i32);")
                //       0    5    10   15   20    25
                .match_decl("returns i32")
                .expect("valid parse")
                .expect("valid decl"),
            canonical
        );
        assert_eq!(
            Parser::new("fn foo(a: i32,);")
                //       0    5    10   15   20    25
                .match_decl("returns i32")
                .expect("valid parse")
                .expect("valid decl"),
            canonical
        );
    }

    #[test]
    fn fn_multi_arg() {
        let canonical = SyntaxDecl::Function {
            name: "foo".to_owned(),
            args: vec![
                FuncArgSyntax {
                    type_: AbiType::I32,
                    name: "a".to_owned(),
                    location: Location { line: 1, column: 7 },
                },
                FuncArgSyntax {
                    type_: AbiType::F64,
                    name: "b".to_owned(),
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
                .match_decl("two args")
                .expect("valid parse")
                .expect("valid decl"),
            canonical
        );
        assert_eq!(
            Parser::new("fn foo(a: i32, b: f64, );")
                //       0    5    10   15   20    25
                .match_decl("two args with trailing comma")
                .expect("valid parse")
                .expect("valid decl"),
            canonical
        );
    }

    #[test]
    fn fn_many_returns() {
        assert_eq!(
            Parser::new("fn getch() -> r1: i32, r2: i64, r3: f32;")
                //       0    5    10   15   20   25   30
                .match_decl("returns u8")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Function {
                name: "getch".to_owned(),
                args: Vec::new(),
                rets: vec![
                    FuncArgSyntax {
                        type_: AbiType::I32,
                        name: "r1".to_owned(),
                        location: Location {
                            line: 1,
                            column: 14,
                        },
                    },
                    FuncArgSyntax {
                        type_: AbiType::I64,
                        name: "r2".to_owned(),
                        location: Location {
                            line: 1,
                            column: 23,
                        },
                    },
                    FuncArgSyntax {
                        type_: AbiType::F32,
                        name: "r3".to_owned(),
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
                "fn fgetch(fptr: i32) -> r: i32 where
file: file_t <- in *fptr,
r: u8 <- out r;"
            )
            //       0    5    10   15   20   25   30
            .match_decl("returns u8")
            .expect("valid parse")
            .expect("valid decl"),
            SyntaxDecl::Function {
                name: "fgetch".to_owned(),
                args: vec![FuncArgSyntax {
                    type_: AbiType::I32,
                    name: "fptr".to_owned(),
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                },],
                rets: vec![FuncArgSyntax {
                    type_: AbiType::I32,
                    name: "r".to_owned(),
                    location: Location {
                        line: 1,
                        column: 24,
                    },
                }],
                bindings: vec![
                    BindingSyntax {
                        name: "file".to_owned(),
                        type_: SyntaxRef::Name {
                            name: "file_t".to_owned(),
                            location: Location { line: 2, column: 6 },
                        },
                        direction: BindDirection::In,
                        from: BindingRefSyntax::Ptr(Box::new(BindingRefSyntax::Name(
                            "fptr".to_owned()
                        ))),
                        location: Location { line: 2, column: 0 },
                    },
                    BindingSyntax {
                        name: "r".to_owned(),
                        type_: SyntaxRef::Atom {
                            atom: AtomType::U8,
                            location: Location { line: 3, column: 3 },
                        },
                        direction: BindDirection::Out,
                        from: BindingRefSyntax::Name("r".to_owned()),
                        location: Location { line: 3, column: 0 },
                    }
                ],
                location: Location { line: 1, column: 0 },
            }
        );
    }
}
