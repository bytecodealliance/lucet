#[macro_use]
extern crate failure;

mod lexer;
mod parser;
mod types;
mod validate;
mod backend;
mod cache;
mod cgenerator;
mod config;
mod data_description_helper;
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
use crate::data_description_helper::DataDescriptionHelper;
use crate::cache::Cache;
use crate::generators::{Generators, Generator};
use crate::pretty_writer::PrettyWriter;
use crate::validate::DataDescription;
use std::io::Write;

pub fn run<W: Write>(config: &Config, input: &str, output: W) -> Result<(), IDLError> {
    let mut parser = Parser::new(&input);
    let decls = parser.match_decls()?;

    let data_description = DataDescription::validate(&decls)?;
    let deps = data_description
        .ordered_dependencies()
        .map_err(|_| IDLError::InternalError("Unable to resolve dependencies"))?;
    let data_description_helper = DataDescriptionHelper { data_description };

    let mut cache = Cache::default();
    let mut generator = Generators::c(config);

    let mut pretty_writer = PrettyWriter::new(output);
    generator.gen_prelude(&mut pretty_writer)?;
    for id in deps {
        data_description_helper.gen_for_id(&mut generator, &mut cache, &mut pretty_writer, id)?;
    }
    Ok(())
}
