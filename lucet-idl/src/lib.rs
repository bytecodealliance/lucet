#![deny(bare_trait_objects)]

#[macro_use]
extern crate failure;

mod atoms;
mod c;
mod config;
mod cursor;
mod error;
mod lexer;
mod parser;
mod prelude;
mod pretty_writer;
mod repr;
mod rust;
mod validate;

pub use crate::config::{Backend, Config};
pub use crate::cursor::*;
pub use crate::error::{IDLError, ValidationError};
pub use crate::{AbiType, AtomType};

use crate::c::CGenerator;
use crate::parser::Parser;
use crate::rust::RustGenerator;
use crate::validate::package_from_declarations;
use lucet_module::bindings::Bindings;
use std::collections::HashMap;
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

pub fn parse_package(input: &str) -> Result<Package, IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;
    let pkg = package_from_declarations(&decls)?;
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

impl Package {
    pub fn bindings(&self) -> Bindings {
        let mut bs = HashMap::new();
        for m in self.modules() {
            let mut mod_bs = HashMap::new();
            for f in m.functions() {
                mod_bs.insert(f.name().to_owned(), f.host_func_name());
            }
            bs.insert(m.name().to_owned(), mod_bs);
        }
        Bindings::new(bs)
    }
}

fn generate_bindings(bindings: &Bindings, mut output: Box<dyn Write>) -> Result<(), IDLError> {
    let bindings_json = bindings
        .to_string()
        .map_err(|_| IDLError::InternalError("bindings generation"))?;
    output.write_all(bindings_json.as_bytes())?;
    Ok(())
}
