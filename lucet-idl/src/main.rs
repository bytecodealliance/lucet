use clap::{App, Arg};
use lucet_idl::{run, Config, IDLError};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process;

#[derive(Clone, Debug)]
pub struct ExeConfig {
    pub input_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub config: Config,
}

impl ExeConfig {
    pub fn parse() -> Result<Self, IDLError> {
        let matches = App::new("lucet-idl")
            .version(env!("CARGO_PKG_VERSION"))
            .about("lucet_idl code generator")
            .arg(
                Arg::with_name("input")
                    .required(true)
                    .help("Path to the input file"),
            )
            .arg(
                Arg::with_name("backend")
                    .short("b")
                    .long("backend")
                    .default_value("c_guest")
                    .takes_value(true)
                    .required(false)
                    .help("Backend, one of: c_guest, rust_guest, rust_host"),
            )
            .arg(
                Arg::with_name("output")
                    .short("o")
                    .takes_value(true)
                    .required(false)
                    .help("output path"),
            )
            .get_matches();
        let input_path = PathBuf::from(
            matches
                .value_of("input")
                .ok_or(IDLError::UsageError("Input file required".to_owned()))?,
        );
        let output_path = matches.value_of("output").map(PathBuf::from);
        let config = Config::parse(matches.value_of("backend").unwrap())?;
        Ok(ExeConfig {
            input_path,
            output_path,
            config,
        })
    }
}
fn doit() -> Result<(), IDLError> {
    let exe_config = ExeConfig::parse()?;
    let mut source = String::new();
    File::open(&exe_config.input_path)?.read_to_string(&mut source)?;

    let output: Box<dyn Write> = match exe_config.output_path {
        Some(ref p) => Box::new(File::create(p)?),
        None => Box::new(io::stdout()),
    };

    run(&exe_config.config, &exe_config.input_path, output)?;

    Ok(())
}

fn main() {
    if let Err(e) = doit() {
        eprintln!("{}", e);
        process::exit(1);
    }
}
