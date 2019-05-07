use crate::parser;
use crate::types::Location;
use std::error::Error;
use std::fmt;
use std::io;

#[allow(dead_code)]
#[derive(Debug, Fail)]
pub enum IDLError {
    #[fail(display = "Internal error: {}", _0)]
    InternalError(&'static str),
    #[fail(display = "Incorrect usage: {}", _0)]
    UsageError(&'static str),
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValidationError {
    NameAlreadyExists {
        name: String,
        at_location: Location,
        previous_location: Location,
    },
    NameNotFound {
        name: String,
        use_location: Location,
    },
    Empty {
        name: String,
        location: Location,
    },
    Infinite {
        name: String,
        location: Location,
    },
    Syntax {
        expected: &'static str,
        location: Location,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::NameAlreadyExists {
                name,
                at_location,
                previous_location,
            } => write!(
                f,
                "Redefinition of name {} at line {} - previous definition was at line {}",
                name, at_location.line, previous_location.line
            ),
            ValidationError::NameNotFound { name, use_location } => {
                write!(f, "Name {} not found at line {}", name, use_location.line)
            }
            ValidationError::Empty { name, location } => {
                write!(f, "Empty definition for {} at line {}", name, location.line)
            }
            ValidationError::Infinite { name, location } => write!(
                f,
                "Circular reference for {} at line {}",
                name, location.line
            ),
            ValidationError::Syntax { expected, location } => write!(
                f,
                "Invalid syntax: expected {} at line {}",
                expected, location.line
            ),
        }
    }
}

impl Error for ValidationError {
    fn description(&self) -> &str {
        "Validation error"
    }
}
