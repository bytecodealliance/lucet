use super::ast::Document;
use super::parser::DeclSyntax;
use crate::Location;

#[derive(Debug, Fail)]
pub enum ValidationError {
    #[fail(display = "Redefinition of name `{}`", name)]
    NameAlreadyExists {
        name: String,
        at_location: Location,
        previous_location: Location,
    },
}

pub fn validate(decls: &[DeclSyntax]) -> Result<Document, ValidationError> {
    unimplemented!()
}
