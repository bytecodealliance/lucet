#[macro_use]
extern crate failure;

mod backend;
mod c;
mod cache;
mod config;
mod errors;
mod generator;
mod lexer;
mod module;
mod parser;
mod pretty_writer;
mod rust;
mod target;
mod types;

pub use crate::backend::{Backend, BackendConfig};
pub use crate::config::Config;
pub use crate::errors::IDLError;
pub use crate::target::Target;

use crate::cache::Cache;
use crate::module::Module;
use crate::parser::Parser;
use crate::pretty_writer::PrettyWriter;
use std::io::Write;

pub fn run<W: Write>(config: &Config, input: &str, output: W) -> Result<W, IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;

    let module = Module::from_declarations(&decls)?;
    let deps = module
        .ordered_dependencies()
        .map_err(|_| IDLError::InternalError("Unable to resolve dependencies"))?;

    let mut cache = Cache::default();
    let mut generator = config.generator();

    let mut pretty_writer = PrettyWriter::new(output);
    generator.gen_prelude(&mut pretty_writer)?;
    for id in deps {
        generator.gen_for_id(&module, &mut cache, &mut pretty_writer, id)?;
    }
    Ok(pretty_writer
        .into_inner()
        .expect("outermost pretty_writer can unwrap"))
}
