use super::lexer::{LexError, Lexer, LocatedError, LocatedToken, Token};
use super::types::{AtomType, Attr, Location};
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyntaxDecl {
    Struct {
        name: String,
        members: Vec<StructMember>,
        attrs: Vec<Attr>,
        location: Location,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
        attrs: Vec<Attr>,
        location: Location,
    },
    Alias {
        name: String,
        what: SyntaxRef,
        attrs: Vec<Attr>,
        location: Location,
    },
    Module {
        name: String,
        decls: Vec<Box<SyntaxDecl>>,
        attrs: Vec<Attr>,
        location: Location,
    }
}

impl SyntaxDecl {
    pub fn name(&self) -> &str {
        match self {
            SyntaxDecl::Struct { name, .. } => &name,
            SyntaxDecl::Enum { name, .. } => &name,
            SyntaxDecl::Alias { name, .. } => &name,
            SyntaxDecl::Module { name, .. } => &name,
        }
    }
    pub fn location(&self) -> &Location {
        match self {
            SyntaxDecl::Struct { location, .. } => &location,
            SyntaxDecl::Enum { location, .. } => &location,
            SyntaxDecl::Alias { location, .. } => &location,
            SyntaxDecl::Module { location, .. } => &location,
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
    pub attrs: Vec<Attr>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UnionVariant {
    pub name: String,
    pub type_: Option<SyntaxRef>,
    pub attrs: Vec<Attr>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub attrs: Vec<Attr>,
    pub location: Location,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParseError {
    pub location: Location,
    pub message: String,
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
            Some(Token::Word(text)) | Some(Token::Keyword(text)) => {
                self.consume();
                Ok(text)
            }
            t => parse_err!(self.location, "{}, got {:?}", err_msg, t),
        }
    }

    fn match_attr_body(&mut self) -> Result<Attr, ParseError> {
        let location = self.location;
        self.match_token(Token::LBracket, "expected attribute start [")?;
        let key = self.match_a_word("expected attribute key")?;
        self.match_token(Token::Equals, "expected =")?;
        let val = match self.token() {
            Some(Token::Word(text)) => text,
            Some(Token::Quote(text)) => text,
            _ => parse_err!(self.location, "expected word or quoted string")?,
        };
        self.consume();
        self.match_token(Token::RBracket, "expected ]")?;
        Ok(Attr::new(key, val, location))
    }

    fn match_struct_body(&mut self) -> Result<Vec<StructMember>, ParseError> {
        let mut members = Vec::new();
        let mut attrs = Vec::new();
        loop {
            match self.token() {
                Some(Token::RBrace) => {
                    self.consume();
                    break;
                }
                Some(Token::Hash) => {
                    self.consume();
                    attrs.push(self.match_attr_body()?);
                }
                Some(Token::Word(member_name)) | Some(Token::Keyword(member_name)) => {
                    let location = self.location;
                    self.consume();
                    self.match_token(Token::Colon, "expected :")?;
                    let member_ref = self.match_ref("expected member type")?;
                    members.push(StructMember {
                        name: member_name.to_string(),
                        type_: member_ref,
                        attrs: attrs.clone(),
                        location,
                    });
                    attrs.clear();
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
                _ => parse_err!(self.location, "in struct body:\nexpected member name or '}'")?,
            }
        }
        Ok(members)
    }

    fn match_enum_body(&mut self) -> Result<Vec<EnumVariant>, ParseError> {
        let mut names = Vec::new();
        let mut attrs = Vec::new();
        loop {
            match self.token() {
                Some(Token::RBrace) => {
                    self.consume();
                    break;
                }
                Some(Token::Hash) => {
                    self.consume();
                    attrs.push(self.match_attr_body()?);
                }
                Some(Token::Word(name)) => {
                    let location = self.location;
                    self.consume();
                    names.push(EnumVariant {
                        name: name.to_owned(),
                        attrs: attrs.clone(),
                        location,
                    });
                    attrs.clear();
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

    pub fn match_decl(&mut self, err_msg: &str) -> Result<Option<SyntaxDecl>, ParseError> {
        let mut attrs = Vec::new();
        loop {
            match self.token() {
                Some(Token::Keyword("struct")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected struct name"))?;
                    err_ctx!(err_msg, self.match_token(Token::LBrace, "expected {"))?;
                    let members = err_ctx!(err_msg, self.match_struct_body())?;
                    return Ok(Some(SyntaxDecl::Struct {
                        name: name.to_owned(),
                        members,
                        attrs,
                        location,
                    }));
                }
                Some(Token::Keyword("enum")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected enum name"))?;
                    err_ctx!(err_msg, self.match_token(Token::LBrace, "expected {"))?;
                    let variants = err_ctx!(err_msg, self.match_enum_body())?;
                    return Ok(Some(SyntaxDecl::Enum {
                        name: name.to_owned(),
                        variants,
                        attrs,
                        location,
                    }));
                }
                Some(Token::Keyword("type")) => {
                    let location = self.location;
                    self.consume();
                    let name = err_ctx!(err_msg, self.match_a_word("expected type name"))?;
                    err_ctx!(err_msg, self.match_token(Token::Equals, "expected ="))?;
                    let what = self.match_ref("type value")?;
                    return Ok(Some(SyntaxDecl::Alias {
                        name: name.to_owned(),
                        what,
                        attrs,
                        location,
                    }));
                }
                Some(Token::Hash) => {
                    self.consume();
                    attrs.push(self.match_attr_body()?);
                    continue;
                }
                Some(_) => return parse_err!(self.location, "expected keyword or attribute"),
                None => return Ok(None),
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
            Some(Token::Word(name)) | Some(Token::Keyword(name)) => {
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
                attrs: Vec::new(),
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
                    attrs: Vec::new(),
                    location: Location {
                        line: 1,
                        column: 12,
                    },
                }],
                attrs: Vec::new(),
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
                    attrs: Vec::new(),
                    location: Location {
                        line: 1,
                        column: 12,
                    },
                }],
                attrs: Vec::new(),
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
                        attrs: Vec::new(),
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
                        attrs: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 19,
                        },
                    },
                ],
                attrs: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        );

    }
    #[test]
    fn struct_empty_one_attribute() {
        // Test out attributes:
        let mut parser = Parser::new("#[key1=val1] struct foo {}");
        assert_eq!(
            parser
                .match_decl("empty struct")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: Vec::new(),
                attrs: vec![Attr::new("key1", "val1", Location { line: 1, column: 0 })],
                location: Location {
                    line: 1,
                    column: 13,
                },
            }
        );
    }
    #[test]
    fn struct_empty_one_attribute_with_spaces() {
        let mut parser = Parser::new("#[key2=\"1 value with spaces!\"]\nstruct foo {}");
        assert_eq!(
            parser
                .match_decl("empty struct")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: Vec::new(),
                attrs: vec![Attr::new(
                    "key2",
                    "1 value with spaces!",
                    Location { line: 1, column: 0 },
                )],
                location: Location { line: 2, column: 0 },
            }
        );
    }
    #[test]
    fn struct_empty_multiple_attributes() {
        let mut parser = Parser::new("#[key1=val1]\n\t#[key2 = \"val2\"   ]\nstruct foo {}");
        assert_eq!(
            parser
                .match_decl("empty struct")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: Vec::new(),
                attrs: vec![
                    Attr::new("key1", "val1", Location { line: 1, column: 0 }),
                    Attr::new("key2", "val2", Location { line: 2, column: 8 }),
                ],
                location: Location { line: 3, column: 0 },
            }
        );
    }
    #[test]
    fn struct_member_attribute() {
        let mut parser = Parser::new("struct foo {\n\t#[key=val]\n\tmem: f32,\n}");
        assert_eq!(
            parser
                .match_decl("empty struct")
                .expect("valid parse")
                .expect("valid decl"),
            SyntaxDecl::Struct {
                name: "foo".to_string(),
                members: vec![StructMember {
                    name: "mem".to_owned(),
                    type_: SyntaxRef::Atom {
                        atom: AtomType::F32,
                        location: Location {
                            line: 3,
                            column: 13,
                        },
                    },
                    attrs: vec![Attr::new("key", "val", Location { line: 2, column: 8 })],
                    location: Location { line: 3, column: 8 },
                }],
                attrs: Vec::new(), //
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
                members: vec![StructMember {
                    name: "a".to_owned(),
                    type_: SyntaxRef::Name {
                        name: "mod".to_owned(),
                        location: Location {
                            line: 1,
                            column: 15,
                        },
                    },
                    attrs: Vec::new(),
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
                    attrs: Vec::new(),
                    location: Location {
                        line: 1,
                        column: 20,
                    },
                }],
                attrs: Vec::new(),
                location: Location { line: 1, column: 0 },
            }
        );
    }
    #[test]
    fn enums() {
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
                attrs: Vec::new(),
                location: Location { line: 1, column: 0 },
            },
        );
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
                    attrs: Vec::new(),
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                }],
                attrs: Vec::new(),
                location: Location { line: 1, column: 0 },
            },
        );
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
                    attrs: Vec::new(),
                    location: Location {
                        line: 1,
                        column: 10,
                    },
                }],
                attrs: Vec::new(),
                location: Location { line: 1, column: 0 },
            },
        );
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
                        attrs: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 11,
                        },
                    },
                    EnumVariant {
                        name: "two".to_owned(),
                        attrs: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 16,
                        },
                    },
                    EnumVariant {
                        name: "three".to_owned(),
                        attrs: Vec::new(),
                        location: Location {
                            line: 1,
                            column: 21,
                        },
                    },
                    EnumVariant {
                        name: "four".to_owned(),
                        attrs: Vec::new(),
                        location: Location { line: 2, column: 2 },
                    },
                ],
                attrs: Vec::new(),
                location: Location { line: 1, column: 0 },
            },
        );
    }
}
