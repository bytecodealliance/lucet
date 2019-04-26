#[macro_use]
extern crate failure;

mod lexer;
mod parser;
mod types;
mod module;
mod backend;
mod cache;
mod cgenerator;
mod config;
mod errors;
mod generators;
mod pretty_writer;
mod rustgenerator;
mod target;

pub use crate::backend::{Backend, BackendConfig};
pub use crate::config::Config;
pub use crate::target::Target;
pub use crate::errors::IDLError;

use crate::parser::Parser;
use crate::cache::Cache;
use crate::generators::{Generators, Generator};
use crate::pretty_writer::PrettyWriter;
use crate::module::Module;
use std::io::Write;

pub fn run<W: Write>(config: &Config, input: &str, output: W) -> Result<(), IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;

    let module = Module::from_declarations(&decls)?;
    let deps = module
        .ordered_dependencies()
        .map_err(|_| IDLError::InternalError("Unable to resolve dependencies"))?;

    let mut cache = Cache::default();
    let mut generator = Generators::c(config);

    let mut pretty_writer = PrettyWriter::new(output);
    generator.gen_prelude(&mut pretty_writer)?;
    for id in deps {
        generator.gen_for_id(&module, &mut cache, &mut pretty_writer, id)?;
    }
    Ok(())
}
