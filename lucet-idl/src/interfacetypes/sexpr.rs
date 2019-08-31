pub use super::lexer::LexError;
use super::lexer::{Lexer, LocatedError, LocatedToken, Token};
use crate::Location;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SExpr<'a> {
    Vec(Vec<SExpr<'a>>, Location),
    Word(&'a str, Location),
    Ident(&'a str, Location),
    Quote(&'a str, Location),
}

impl<'a> SExpr<'a> {
    pub fn location(&self) -> Location {
        match self {
            SExpr::Vec(_, loc) => *loc,
            SExpr::Word(_, loc) => *loc,
            SExpr::Ident(_, loc) => *loc,
            SExpr::Quote(_, loc) => *loc,
        }
    }

    pub fn is_word(&self, w: &str) -> bool {
        match self {
            SExpr::Word(word, _) => *word == w,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SExprParseError {
    Lex(LexError, Location),
    UnexpectedCloseParen(Location),
    UnexpectedEof,
}

pub struct SExprParser<'a> {
    lex: Lexer<'a>,
    lookahead: Option<Token<'a>>,
    location: Location,
}

impl<'a> SExprParser<'a> {
    pub fn new(text: &'a str) -> SExprParser<'_> {
        SExprParser {
            lex: Lexer::new(text),
            lookahead: None,
            location: Location { line: 0, column: 0 },
        }
    }
    fn consume(&mut self) -> Token<'a> {
        self.lookahead.take().expect("no token to consume")
    }
    fn token(&mut self) -> Result<Option<Token<'a>>, SExprParseError> {
        while self.lookahead == None {
            match self.lex.next() {
                Some(Ok(LocatedToken { token, location })) => {
                    self.location = location;
                    self.lookahead = Some(token)
                }
                Some(Err(LocatedError { error, location })) => {
                    self.location = location;
                    Err(SExprParseError::Lex(error, location))?;
                }
                None => break,
            }
        }
        Ok(self.lookahead)
    }

    pub fn match_sexpr(&mut self) -> Result<SExpr<'a>, SExprParseError> {
        let location = self.location;
        match self.token()? {
            Some(Token::LPar) => {
                self.consume();
                let mut members = Vec::new();
                loop {
                    match self.token()? {
                        Some(Token::RPar) => {
                            self.consume();
                            break;
                        }
                        _ => {
                            members.push(self.match_sexpr()?);
                        }
                    }
                }
                Ok(SExpr::Vec(members, location))
            }
            Some(Token::Word(word)) => {
                self.consume();
                Ok(SExpr::Word(word, location))
            }
            Some(Token::Ident(id)) => {
                self.consume();
                Ok(SExpr::Ident(id, location))
            }
            Some(Token::Quote(q)) => {
                self.consume();
                Ok(SExpr::Quote(q, location))
            }
            Some(Token::RPar) => Err(SExprParseError::UnexpectedCloseParen(location)),
            None => Err(SExprParseError::UnexpectedEof),
        }
    }

    pub fn match_sexprs(&mut self) -> Result<Vec<SExpr<'a>>, SExprParseError> {
        let mut sexprs = Vec::new();
        while self.token()?.is_some() {
            sexprs.push(self.match_sexpr()?);
        }
        Ok(sexprs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut parser = SExprParser::new("");
        assert_eq!(parser.match_sexprs().expect("valid parse"), Vec::new());
        let mut parser = SExprParser::new("   ;; just a comment\n;;another");
        assert_eq!(parser.match_sexprs().expect("valid parse"), Vec::new());
    }

    #[test]
    fn atoms() {
        let mut parser = SExprParser::new("hello\n$world\n\"a quotation\"");
        assert_eq!(
            parser.match_sexprs().expect("valid parse"),
            vec![
                SExpr::Word("hello", Location { line: 1, column: 0 }),
                SExpr::Ident("world", Location { line: 2, column: 0 }),
                SExpr::Quote("a quotation", Location { line: 3, column: 0 }),
            ]
        );
    }

    #[test]
    fn lists() {
        let mut parser = SExprParser::new("()");
        assert_eq!(
            parser.match_sexprs().expect("valid parse"),
            vec![SExpr::Vec(vec![], Location { line: 1, column: 0 })]
        );

        let mut parser = SExprParser::new("(hello\n$world\n\"a quotation\")");
        assert_eq!(
            parser.match_sexprs().expect("valid parse"),
            vec![SExpr::Vec(
                vec![
                    SExpr::Word("hello", Location { line: 1, column: 1 }),
                    SExpr::Ident("world", Location { line: 2, column: 0 }),
                    SExpr::Quote("a quotation", Location { line: 3, column: 0 }),
                ],
                Location { line: 1, column: 0 }
            )]
        );

        let mut parser = SExprParser::new("((($deep)))");
        assert_eq!(
            parser.match_sexprs().expect("valid parse"),
            vec![SExpr::Vec(
                vec![SExpr::Vec(
                    vec![SExpr::Vec(
                        vec![SExpr::Ident("deep", Location { line: 1, column: 3 })],
                        Location { line: 1, column: 2 }
                    )],
                    Location { line: 1, column: 1 }
                )],
                Location { line: 1, column: 0 }
            )]
        );
    }

    #[test]
    fn errors() {
        let mut parser = SExprParser::new("(");
        assert_eq!(
            parser.match_sexprs().err().expect("dies"),
            SExprParseError::UnexpectedEof,
        );
        let mut parser = SExprParser::new(")");
        assert_eq!(
            parser.match_sexprs().err().expect("dies"),
            SExprParseError::UnexpectedCloseParen(Location { line: 1, column: 0 })
        );
        let mut parser = SExprParser::new("())");
        assert_eq!(
            parser.match_sexprs().err().expect("dies"),
            SExprParseError::UnexpectedCloseParen(Location { line: 1, column: 2 })
        );
        let mut parser = SExprParser::new("$ ;; should be a lex error");
        assert_eq!(
            parser.match_sexprs().err().expect("dies"),
            SExprParseError::Lex(LexError::EmptyIdentifier, Location { line: 1, column: 0 },),
        );
    }
}
