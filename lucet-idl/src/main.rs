#[macro_use]
extern crate failure;

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

use self::cache::*;
use self::generators::*;

use self::config::*;
use self::data_description_helper::*;
use self::errors::*;
use self::pretty_writer::*;
use lucet_idl::parser::*;
use lucet_idl::validate::*;
use std::fs::File;
use std::io;
use std::io::prelude::*;

fn doit() -> Result<(), IDLError> {
    let config = Config::parse()?;
    let mut source = String::new();
    File::open(&config.input_path)?.read_to_string(&mut source)?;
    let mut parser = Parser::new(&source);
    let decls = parser.match_decls()?;
    let data_description = DataDescription::validate(&decls)?;
    let deps = data_description
        .ordered_dependencies()
        .map_err(|_| IDLError::InternalError("Unable to resolve dependencies"))?;
    let data_description_helper = DataDescriptionHelper { data_description };
    let mut cache = Cache::default();
    let mut generator = Generators::c(&config);
    let mut pretty_writer = PrettyWriter::new(io::stdout());
    generator.gen_prelude(&mut pretty_writer)?;
    for id in deps {
        data_description_helper.gen_for_id(&mut generator, &mut cache, &mut pretty_writer, id)?;
    }
    Ok(())
}

fn main() {
    doit().unwrap();
}
