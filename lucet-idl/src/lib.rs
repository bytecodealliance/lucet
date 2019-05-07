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
use crate::pretty_writer::PrettyWriter;
use std::io::Write;

pub fn run<W: Write>(config: &Config, input: &str, output: W) -> Result<W, IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;

    let mut pretty_writer = PrettyWriter::new(output);
    let pkg = Package::from_declarations(&decls)?;

    for (_ident, mod_) in pkg.modules {
        let deps = mod_
            .ordered_dependencies()
            .map_err(|_| IDLError::InternalError("Unable to resolve dependencies"))?;

        let mut generator = config.generator();

        generator.gen_prelude(&mut pretty_writer)?;
        for id in deps {
            generator.gen_for_id(&mod_, &mut pretty_writer, id)?;
        }
    }
    Ok(pretty_writer
        .into_inner()
        .expect("outermost pretty_writer can unwrap"))
}
