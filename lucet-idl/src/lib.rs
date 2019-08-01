#![deny(bare_trait_objects)]

#[macro_use]
extern crate failure;

mod atoms;
mod cursor;
mod prelude;
mod repr;
mod validate;
//mod c;
mod config;
pub mod env;
mod error;
mod lexer;
mod parser;
mod pretty_writer;
//mod rust;

pub use crate::config::{Backend, Config};
pub use crate::cursor::*;
pub use crate::error::{IDLError, ValidationError};
pub use crate::repr::{BindingDirection, PackageRepr};
pub use crate::{AbiType, AtomType};

//use crate::c::CGenerator;
use crate::parser::Parser;
//use crate::rust::RustGenerator;
use crate::validate::package_from_declarations;
use lucet_module::bindings::Bindings;
use std::io::Write;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

pub trait MemArea {
    fn mem_size(&self) -> usize;
    fn mem_align(&self) -> usize;
}

pub fn parse_package(input: &str) -> Result<PackageRepr, IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;
    let pkg = package_from_declarations(&decls)?;
    Ok(pkg)
}

pub fn codegen(package: &Package, config: &Config, output: Box<dyn Write>) -> Result<(), IDLError> {
    match config.backend {
        Backend::CGuest => unimplemented!(), //CGenerator::new(output).generate_guest(package)?,
        Backend::RustGuest => unimplemented!(), //RustGenerator::new(output).generate_guest(package)?,
        Backend::RustHost => unimplemented!(), //RustGenerator::new(output).generate_host(package)?,
        Backend::Bindings => unimplemented!(), //generate_bindings(&package.bindings(), output)?,
    }
    Ok(())
}

pub fn run(config: &Config, input: &str, output: Box<dyn Write>) -> Result<(), IDLError> {
    let pkg_repr = parse_package(input)?;
    let pkg = Package::new(&pkg_repr);
    codegen(&pkg, config, output)
}

fn generate_bindings(bindings: &Bindings, mut output: Box<dyn Write>) -> Result<(), IDLError> {
    let bindings_json = bindings
        .to_string()
        .map_err(|_| IDLError::InternalError("bindings generation"))?;
    output.write_all(bindings_json.as_bytes())?;
    Ok(())
}
