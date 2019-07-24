#![deny(bare_trait_objects)]

#[macro_use]
extern crate failure;

mod c;
mod config;
mod data_layout;
mod error;
mod function;
mod lexer;
mod module;
mod package;
mod parser;
mod pretty_writer;
mod rust;
mod types;

pub use crate::config::{Backend, Config};
pub use crate::error::IDLError;
pub use crate::function::{BindingRef, FuncArg, FuncBinding, FuncDecl, ParamPosition};
pub use crate::module::Module;
pub use crate::package::Package;
pub use crate::types::{
    AbiType, AliasDataType, AtomType, DataType, DataTypeRef, DataTypeVariant, EnumDataType, Ident,
    Location, MemArea, Name, Named, StructDataType, StructMember,
};

use crate::c::CGenerator;
use crate::parser::Parser;
use crate::rust::RustGenerator;
use lucet_module::bindings::Bindings;
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
        Backend::Bindings => generate_bindings(&package.bindings(), output)?,
    }
    Ok(())
}

pub fn run(config: &Config, input: &str, output: Box<dyn Write>) -> Result<(), IDLError> {
    let pkg = parse_package(input)?;
    codegen(&pkg, config, output)
}

fn generate_bindings(bindings: &Bindings, mut output: Box<dyn Write>) -> Result<(), IDLError> {
    let bindings_json = bindings
        .to_string()
        .map_err(|_| IDLError::InternalError("bindings generation"))?;
    output.write_all(bindings_json.as_bytes())?;
    Ok(())
}
