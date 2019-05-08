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

pub use crate::backend::{Backend, BackendConfig};
pub use crate::config::Config;
pub use crate::error::IDLError;
pub use crate::target::Target;

use crate::package::Package;
use crate::parser::Parser;
use std::io::Write;

pub fn run(config: &Config, input: &str, output: Box<dyn Write>) -> Result<(), IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;

    let pkg = Package::from_declarations(&decls)?;

    let mut generator = config.generator(output);

    for (_ident, mod_) in pkg.modules {
        let deps = mod_
            .ordered_datatype_idents()
            .map_err(|_| IDLError::InternalError("Unable to resolve dependencies"))?;

        for id in deps {
            generator.gen_for_id(&mod_, id)?;
        }
    }
    Ok(())
}
