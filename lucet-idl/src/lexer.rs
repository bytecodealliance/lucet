use crate::Location;
use std::str::CharIndices;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token<'a> {
    LPar,     // (
    RPar,     // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    Star,     // *
    Colon,    // :
    Semi,     // ;
    Comma,    // ,
    Hash,     // #
    Equals,   // =
    LArrow,   // <-
    RArrow,   // ->
    Word(&'a str),
    Quote(&'a str), // Found between balanced "". No escaping.
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LocatedToken<'a> {
    pub token: Token<'a>,
    pub location: Location,
}

fn token(token: Token<'_>, location: Location) -> Result<LocatedToken<'_>, LocatedError> {
    Ok(LocatedToken { token, location })
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LexError {
    InvalidChar(char),
    UnterminatedComment,
    UnterminatedQuote,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LocatedError {
    pub error: LexError,
    pub location: Location,
}

fn error<'a>(error: LexError, location: Location) -> Result<LocatedToken<'a>, LocatedError> {
    Err(LocatedError { error, location })
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: CharIndices<'a>,
    lookahead: Option<char>,
    pos: usize,
    line_number: usize,
    column_start: usize,
    tab_compensation: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Lexer<'_> {
        let mut lex = Lexer {
            source: s,
            chars: s.char_indices(),
            lookahead: None,
            pos: 0,
            line_number: 1,
            column_start: 0,
            tab_compensation: 0,
        };
        lex.next_ch();
        lex
    }

    fn next_ch(&mut self) -> Option<char> {
        if self.lookahead == Some('\n') {
            self.line_number += 1;
            self.column_start = self.pos + 1; // Next column starts a fresh line
            self.tab_compensation = 0;
        } else if self.lookahead == Some('\t') {
            self.tab_compensation += 7; // One column for the position of the char itself, add 7 more for a tabwidth of 8
        }
        match self.chars.next() {
            Some((idx, ch)) => {
                self.pos = idx;
                self.lookahead = Some(ch);
            }
            None => {
                self.pos = self.source.len();
                self.lookahead = None;
            }
        }
        self.lookahead
    }

    fn loc(&self) -> Location {
        Location {
            line: self.line_number,
            column: self.pos - self.column_start + self.tab_compensation,
        }
    }

    fn looking_at(&self, prefix: &str) -> bool {
        self.source[self.pos..].starts_with(prefix)
    }

    fn scan_char(&mut self, tok: Token<'a>) -> Result<LocatedToken<'a>, LocatedError> {
        assert!(self.lookahead.is_some());
        let loc = self.loc();
        self.next_ch();
        token(tok, loc)
    }

    pub fn rest_of_line(&mut self) -> &'a str {
        let begin = self.pos;
        loop {
            match self.next_ch() {
                None | Some('\n') => return &self.source[begin..self.pos],
                _ => {}
            }
        }
    }

    fn scan_word(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let begin = self.pos;
        let loc = self.loc();
        assert!(self.lookahead == Some('_') || self.lookahead.unwrap().is_alphabetic());
        loop {
            match self.next_ch() {
                Some('_') => {}
                Some(ch) if ch.is_alphanumeric() => {}
                _ => break,
            }
        }
        let text = &self.source[begin..self.pos];
        token(Token::Word(text), loc)
    }

    fn scan_comment(&mut self) -> Result<(), LocatedError> {
        assert!(self.lookahead == Some('/'));
        let loc = self.loc();
        loop {
            match self.next_ch() {
                None => Err(LocatedError {
                    error: LexError::UnterminatedComment,
                    location: loc,
                })?,
                Some('*') => {
                    if self.looking_at("*/") {
                        self.next_ch(); // Consume the slash
                        self.next_ch(); // Move to next token for outer loop
                        break;
                    }
                }
                Some(_) => {}
            }
        }
        Ok(())
    }

    fn scan_quote(&mut self) -> Result<LocatedToken<'a>, LocatedError> {
        let begin = self.pos;
        let loc = self.loc();
        assert!(self.lookahead == Some('"'));
        loop {
            match self.next_ch() {
                None => Err(LocatedError {
                    error: LexError::UnterminatedQuote,
                    location: loc,
                })?,
                Some('"') => {
                    self.next_ch();
                    break;
                }
                _ => {}
            }
        }
        let text = &self.source[(begin + 1)..(self.pos - 1)];
        token(Token::Quote(text), loc)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<LocatedToken<'a>, LocatedError>> {
        loop {
            let loc = self.loc();
            return match self.lookahead {
                None => None,
                Some(c) => Some(match c {
                    '(' => self.scan_char(Token::LPar),
                    ')' => self.scan_char(Token::RPar),
                    '{' => self.scan_char(Token::LBrace),
                    '}' => self.scan_char(Token::RBrace),
                    '[' => self.scan_char(Token::LBracket),
                    ']' => self.scan_char(Token::RBracket),
                    '*' => self.scan_char(Token::Star),
                    ':' => self.scan_char(Token::Colon),
                    ';' => self.scan_char(Token::Semi),
                    ',' => self.scan_char(Token::Comma),
                    '#' => self.scan_char(Token::Hash),
                    '=' => self.scan_char(Token::Equals),
                    '-' => {
                        if self.looking_at("->") {
                            self.next_ch(); // Consume -
                            self.next_ch(); // Consume >
                            token(Token::RArrow, loc)
                        } else {
                            self.next_ch();
                            error(LexError::InvalidChar('-'), loc)
                        }
                    }
                    '<' => {
                        if self.looking_at("<-") {
                            self.next_ch(); // Consume <
                            self.next_ch(); // Consume -
                            token(Token::LArrow, loc)
                        } else {
                            self.next_ch();
                            error(LexError::InvalidChar('<'), loc)
                        }
                    }
                    '/' => {
                        if self.looking_at("//") {
                            self.rest_of_line();
                            continue;
                        } else if self.looking_at("/*") {
                            match self.scan_comment() {
                                Ok(()) => continue,
                                Err(e) => return Some(Err(e)),
                            }
                        } else {
                            self.next_ch();
                            error(LexError::InvalidChar('/'), loc)
                        }
                    }
                    '"' => self.scan_quote(),
                    ch if ch.is_alphabetic() => self.scan_word(),
                    ch if ch.is_whitespace() => {
                        self.next_ch();
                        continue;
                    }
                    _ => {
                        self.next_ch();
                        error(LexError::InvalidChar(c), loc)
                    }
                }),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(
        token: Token<'_>,
        line: usize,
        column: usize,
    ) -> Option<Result<LocatedToken<'_>, LocatedError>> {
        Some(super::token(token, Location { line, column }))
    }

    fn error<'a>(
        err: LexError,
        line: usize,
        column: usize,
    ) -> Option<Result<LocatedToken<'a>, LocatedError>> {
        Some(super::error(err, Location { line, column }))
    }

    #[test]
    fn comments() {
        let mut lex = Lexer::new("the quick // brown fox\njumped\n//over the three\nlazy//dogs");
        assert_eq!(lex.next(), token(Token::Word("the"), 1, 0));
        assert_eq!(lex.next(), token(Token::Word("quick"), 1, 4));
        assert_eq!(lex.next(), token(Token::Word("jumped"), 2, 0));
        assert_eq!(lex.next(), token(Token::Word("lazy"), 4, 0));
        assert_eq!(lex.next(), None);

        let mut lex = Lexer::new("line1 //\nsym_2/#\n\t\tl3///333");
        assert_eq!(lex.next(), token(Token::Word("line1"), 1, 0));
        assert_eq!(lex.next(), token(Token::Word("sym_2"), 2, 0));
        assert_eq!(lex.next(), error(LexError::InvalidChar('/'), 2, 5));
        assert_eq!(lex.next(), token(Token::Hash, 2, 6));
        assert_eq!(lex.next(), token(Token::Word("l3"), 3, 16)); // Two tabs = 16 columns
        assert_eq!(lex.next(), None);

        let mut lex = Lexer::new("a /* b */ c");
        assert_eq!(lex.next(), token(Token::Word("a"), 1, 0));
        assert_eq!(lex.next(), token(Token::Word("c"), 1, 10));

        let mut lex = Lexer::new("a /* b \n*/ c\n/*");
        assert_eq!(lex.next(), token(Token::Word("a"), 1, 0));
        assert_eq!(lex.next(), token(Token::Word("c"), 2, 3));
        assert_eq!(lex.next(), error(LexError::UnterminatedComment, 3, 0));
    }

    #[test]
    fn quotes() {
        let mut lex = Lexer::new("a \"bc\" d");
        assert_eq!(lex.next(), token(Token::Word("a"), 1, 0));
        assert_eq!(lex.next(), token(Token::Quote("bc"), 1, 2));
        assert_eq!(lex.next(), token(Token::Word("d"), 1, 7));

        let mut lex = Lexer::new("a \"b\nc\" d");
        assert_eq!(lex.next(), token(Token::Word("a"), 1, 0));
        assert_eq!(lex.next(), token(Token::Quote("b\nc"), 1, 2));
        assert_eq!(lex.next(), token(Token::Word("d"), 2, 3));

        let mut lex = Lexer::new("a \"b");
        assert_eq!(lex.next(), token(Token::Word("a"), 1, 0));
        assert_eq!(lex.next(), error(LexError::UnterminatedQuote, 1, 2));
    }
    #[test]
    fn punctuation() {
        let mut lex = Lexer::new("{} () [] *#=:,;");
        assert_eq!(lex.next(), token(Token::LBrace, 1, 0));
        assert_eq!(lex.next(), token(Token::RBrace, 1, 1));
        assert_eq!(lex.next(), token(Token::LPar, 1, 3));
        assert_eq!(lex.next(), token(Token::RPar, 1, 4));
        assert_eq!(lex.next(), token(Token::LBracket, 1, 6));
        assert_eq!(lex.next(), token(Token::RBracket, 1, 7));
        assert_eq!(lex.next(), token(Token::Star, 1, 9));
        assert_eq!(lex.next(), token(Token::Hash, 1, 10));
        assert_eq!(lex.next(), token(Token::Equals, 1, 11));
        assert_eq!(lex.next(), token(Token::Colon, 1, 12));
        assert_eq!(lex.next(), token(Token::Comma, 1, 13));
        assert_eq!(lex.next(), token(Token::Semi, 1, 14));
        assert_eq!(lex.next(), None);
    }

    #[test]
    fn arrows() {
        let mut lex = Lexer::new("<-->\n<- ->");
        assert_eq!(lex.next(), token(Token::LArrow, 1, 0));
        assert_eq!(lex.next(), token(Token::RArrow, 1, 2));
        assert_eq!(lex.next(), token(Token::LArrow, 2, 0));
        assert_eq!(lex.next(), token(Token::RArrow, 2, 3));
        assert_eq!(lex.next(), None);
    }
}
