#![deny(bare_trait_objects)]

#[macro_use]
extern crate failure;

mod backend;
mod c;
mod config;
mod error;
mod generator;
mod lexer;
mod module;
mod package;
mod parser;
mod pretty_writer;
mod rust;
mod target;
mod types;

pub use crate::backend::Backend;
pub use crate::config::Config;
pub use crate::error::IDLError;
pub use crate::module::Module;
pub use crate::package::Package;
pub use crate::target::Target;
pub use crate::types::{
    AtomType, Attr, DataType, DataTypeRef, FuncDecl, FuncRet, Ident, Location, Name, Named,
};

use crate::parser::Parser;
use std::io::Write;

pub fn parse_package(input: &str) -> Result<Package, IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;
    let pkg = Package::from_declarations(&decls)?;
    Ok(pkg)
}

pub fn codegen(package: &Package, config: &Config, output: Box<dyn Write>) -> Result<(), IDLError> {
    let mut generator = config.generator(output);

    for (_ident, mod_) in package.modules.iter() {
        for dt in mod_.datatypes() {
            generator.gen_datatype(mod_, &dt)?;
        }
        for fdecl in mod_.func_decls() {
            generator.gen_function(mod_, &fdecl)?;
        }
    }
    Ok(())
}

pub fn run(config: &Config, input: &str, output: Box<dyn Write>) -> Result<(), IDLError> {
    let pkg = parse_package(input)?;
    codegen(&pkg, config, output)
}
