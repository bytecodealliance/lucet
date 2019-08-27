use crate::parser;
use crate::Location;
use std::io;

#[derive(Debug, Fail)]
pub enum IDLError {
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    #[fail(display = "Incorrect usage: {}", _0)]
    UsageError(String),
    #[fail(display = "{}", _0)]
    ParseError(#[cause] parser::ParseError),
    #[fail(display = "{}", _0)]
    ValidationError(#[cause] ValidationError),
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
}

impl From<io::Error> for IDLError {
    fn from(e: io::Error) -> Self {
        IDLError::Io(e)
    }
}

impl From<parser::ParseError> for IDLError {
    fn from(e: parser::ParseError) -> Self {
        IDLError::ParseError(e)
    }
}

impl From<ValidationError> for IDLError {
    fn from(e: ValidationError) -> Self {
        IDLError::ValidationError(e)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Fail)]
pub enum ValidationError {
    #[fail(display = "Redefinition of name `{}`", name)]
    NameAlreadyExists {
        name: String,
        at_location: Location,
        previous_location: Location,
    },
    #[fail(display = "Use of unknown name `{}`", name)]
    NameNotFound {
        name: String,
        use_location: Location,
    },
    #[fail(display = "Empty definition for `{}`", name)]
    Empty { name: String, location: Location },
    #[fail(display = "Infinite definition for `{}`", name)]
    Infinite { name: String, location: Location },
    #[fail(display = "Syntax error: expected {}", expected)]
    Syntax {
        expected: &'static str,
        location: Location,
    },
    #[fail(display = "Name `{}` bound to another sort", name)]
    NameSortError {
        name: String,
        use_location: Location,
        bound_location: Location,
    },
    #[fail(display = "Name `{}` already bound", name)]
    BindingNameAlreadyBound {
        name: String,
        at_location: Location,
        bound_location: Location,
    },
    #[fail(display = "Binding type error: expected {}", expected)]
    BindingTypeError {
        expected: &'static str,
        location: Location,
    },
}
