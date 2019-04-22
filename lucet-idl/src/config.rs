use super::backend::*;
use super::errors::*;
use super::target::*;
use clap::{App, Arg};
use std::path::PathBuf;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub target: Target,
    pub backend: Backend,
    pub backend_config: BackendConfig,
}

impl Config {
    pub fn parse() -> Result<Self, IDLError> {
        let matches = App::new("lucet-idl")
            .version("1.0")
            .about("lucet_idl code generator")
            .arg(
                Arg::with_name("input_file")
                    .short("i")
                    .long("input")
                    .takes_value(true)
                    .required(true)
                    .help("Path to the input file"),
            )
            .arg(
                Arg::with_name("target")
                    .short("t")
                    .long("target")
                    .default_value("Generic")
                    .takes_value(true)
                    .required(false)
                    .help("Target, one of: x86, x86_64, x86_64_32, generic"),
            )
            .arg(
                Arg::with_name("backend")
                    .short("b")
                    .long("backend")
                    .default_value("c")
                    .takes_value(true)
                    .required(false)
                    .help("Backend, one of: c, rust"),
            )
            .arg(
                Arg::with_name("zero-native-pointers")
                    .short("z")
                    .long("zero-native-pointers")
                    .takes_value(false)
                    .required(false)
                    .help("Do not serialize native pointers"),
            )
            .get_matches();
        let input_path = PathBuf::from(
            matches
                .value_of("input_file")
                .ok_or(IDLError::UsageError("Input file required"))?,
        );
        let mut target = Target::from(matches.value_of("target").unwrap());
        let backend = Backend::from(matches.value_of("backend").unwrap());
        let zero_native_pointers = matches.is_present("zero-native-pointers");
        if zero_native_pointers {
            target = Target::Generic;
        }
        let backend_config = BackendConfig {
            zero_native_pointers,
        };
        Ok(Config {
            input_path,
            output_path: None,
            target,
            backend,
            backend_config,
        })
    }
}
