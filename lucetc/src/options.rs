use clap::{App, Arg, ArgMatches};
use failure::Error;
use lucetc::{OptLevel, HeapSettings};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenOutput {
    Clif,
    Obj,
    SharedObj,
}

fn parse_humansized(desc: &str) -> Result<u64, Error> {
    use human_size::{Byte, ParsingError, Size, SpecificSize};
    match desc.parse::<Size>() {
        Ok(s) => {
            let bytes: SpecificSize<Byte> = s.into();
            Ok(bytes.value() as u64)
        }
        Err(ParsingError::MissingMultiple) => Ok(desc.parse::<u64>()?),
        Err(e) => Err(e)?,
    }
}

fn humansized(bytes: u64) -> String {
    use human_size::{Byte, Mebibyte, SpecificSize};
    let bytes = SpecificSize::new(bytes as f64, Byte).expect("bytes");
    let mb: SpecificSize<Mebibyte> = bytes.into();
    mb.to_string()
}

#[derive(Debug)]
pub struct Options {
    pub output: PathBuf,
    pub input: Vec<PathBuf>,
    pub codegen: CodegenOutput,
    pub binding_files: Vec<PathBuf>,
    pub builtins_path: Option<PathBuf>,
    pub reserved_size: Option<u64>,
    pub guard_size: Option<u64>,
    pub opt_level: OptLevel,
}

impl Options {
    pub fn from_args(m: &ArgMatches) -> Result<Self, Error> {
        let input: Vec<PathBuf> = m
            .values_of("input")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();

        let output = PathBuf::from(m.value_of("output").unwrap_or("a.out"));

        let binding_files: Vec<PathBuf> = m
            .values_of("bindings")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();

        let codegen = match m.value_of("emit") {
            None => CodegenOutput::SharedObj,
            Some("clif") => CodegenOutput::Clif,
            Some("obj") => CodegenOutput::Obj,
            Some("so") => CodegenOutput::SharedObj,
            Some(_) => panic!("unknown value for emit"),
        };

        let builtins_path = m.value_of("builtins").map(PathBuf::from);

        let reserved_size = if let Some(reserved_str) = m.value_of("reserved_size") {
            Some(parse_humansized(reserved_str)?)
        } else {
            None
        };

        let guard_size = if let Some(guard_str) = m.value_of("guard_size") {
            Some(parse_humansized(guard_str)?)
        } else {
            None
        };

        let opt_level = match m.value_of("opt_level") {
            None => OptLevel::Default,
            Some("default") => OptLevel::Default,
            Some("best") => OptLevel::Best,
            Some("fastest") => OptLevel::Fastest,
            Some(_) => panic!("unknown value for opt-level"),
        };

        Ok(Options {
            output,
            input,
            codegen,
            binding_files,
            builtins_path,
            reserved_size,
            guard_size,
            opt_level,
        })
    }
    pub fn get() -> Result<Self, Error> {
        let m = App::new("lucetc")
            .arg(
                Arg::with_name("precious")
                    .long("--precious")
                    .takes_value(true)
                    .help("directory to keep intermediate build artifacts in"),
            )
            .arg(
                Arg::with_name("emit")
                    .long("emit")
                    .takes_value(true)
                    .possible_values(&["obj", "so", "clif"])
                    .help("type of code to generate (default: so)"),
            )
            .arg(
                Arg::with_name("output")
                    .short("o")
                    .takes_value(true)
                    .multiple(false)
                    .help("output destination, defaults to a.out if unspecified"),
            )
            .arg(
                Arg::with_name("bindings")
                    .long("--bindings")
                    .takes_value(true)
                    .multiple(true)
                    .help("path to bindings json file"),
            )
            .arg(
                Arg::with_name("reserved_size")
                    .long("--reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help(&format!(
                        "size of usable linear memory region. must be multiple of 4k. default: {}",
                        humansized(HeapSettings::default().reserved_size)
                    )),
            )
            .arg(
                Arg::with_name("guard_size")
                    .long("--guard-size")
                    .takes_value(true)
                    .multiple(false)
                    .help(&format!(
                        "size of linear memory guard. must be multiple of 4k. default: {}",
                        humansized(HeapSettings::default().guard_size)
                    )),
            )
            .arg(
                Arg::with_name("builtins")
                    .long("--builtins")
                    .takes_value(true)
                    .help("builtins file"),
            )
            .arg(
                Arg::with_name("input")
                    .multiple(false)
                    .required(true)
                    .help("input file"),
            )
            .arg(
                Arg::with_name("opt_level")
                    .long("--opt-level")
                    .takes_value(true)
                    .possible_values(&["default", "fastest", "best"])
                    .help("optimization level (default: 'default')"),
            )
            .get_matches();

        Self::from_args(&m)
    }
}
