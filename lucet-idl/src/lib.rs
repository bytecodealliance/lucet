#![deny(bare_trait_objects)]

#[macro_use]
extern crate failure;

mod c;
mod config;
mod data_layout;
mod error;
mod lexer;
mod module;
mod package;
mod parser;
mod pretty_writer;
mod rust;
mod types;

pub use crate::config::{Backend, Config};
pub use crate::error::IDLError;
pub use crate::module::Module;
pub use crate::package::Package;
pub use crate::types::{
    AtomType, Attr, DataType, DataTypeRef, FuncDecl, FuncRet, Ident, Location, Name, Named,
};

use crate::c::CGenerator;
use crate::parser::Parser;
use crate::rust::RustGenerator;
use std::io::Write;

pub fn parse_package(input: &str) -> Result<Package, IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;
    let pkg = Package::from_declarations(&decls)?;
    Ok(pkg)
}

pub fn codegen(package: &Package, config: &Config, output: Box<dyn Write>) -> Result<(), IDLError> {
    match config.backend {
        Backend::CGuest => CGenerator::new(output).generate_guest(package)?,
        Backend::RustGuest => RustGenerator::new(output).generate_guest(package)?,
        Backend::RustHost => RustGenerator::new(output).generate_host(package)?,
    }

    Ok(())
}

pub fn run(config: &Config, input: &str, output: Box<dyn Write>) -> Result<(), IDLError> {
    let pkg = parse_package(input)?;
    codegen(&pkg, config, output)
}
