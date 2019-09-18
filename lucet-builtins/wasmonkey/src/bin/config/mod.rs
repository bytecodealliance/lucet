use crate::{PatcherConfig, WError};
use clap::{App, Arg};
use std::path::PathBuf;

#[derive(Default, Clone, Debug)]
pub struct Config {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub patcher_config: PatcherConfig,
}

impl Config {
    pub fn parse_cmdline() -> Result<Self, WError> {
        let matches = App::new("wasmonkey")
            .version("1.0")
            .about("Transforms WASM exports to imports")
            .arg(
                Arg::with_name("input_file")
                    .short("i")
                    .long("input")
                    .takes_value(true)
                    .required(true)
                    .help("Path to the input file"),
            )
            .arg(
                Arg::with_name("output_file")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .required(true)
                    .help("Path to the output file"),
            )
            .arg(
                Arg::with_name("builtins_file")
                    .short("b")
                    .long("builtins")
                    .takes_value(true)
                    .required(false)
                    .help("Path to the builtins library"),
            )
            .arg(
                Arg::with_name("builtins_additional")
                    .short("B")
                    .long("builtins-additional")
                    .takes_value(true)
                    .required(false)
                    .multiple(true)
                    .help("Additional builtins function names to replace"),
            )
            .arg(
                Arg::with_name("builtins_map_file")
                    .short("m")
                    .long("builtins-map")
                    .takes_value(true)
                    .required(false)
                    .help("Path to the builtins map file"),
            )
            .arg(
                Arg::with_name("builtins_map_original_names")
                    .short("n")
                    .long("original-names")
                    .takes_value(false)
                    .required(false)
                    .help("Use the original name as a key in the builtins map"),
            )
            .get_matches();
        let input_path = PathBuf::from(
            matches
                .value_of("input_file")
                .ok_or(WError::UsageError("Input file required"))?,
        );
        let output_path = PathBuf::from(
            matches
                .value_of("output_file")
                .ok_or(WError::UsageError("Output file required"))?,
        );
        let builtins_path = matches.value_of("builtins_file").map(PathBuf::from);
        let builtins_map_path = matches.value_of("builtins_map_file").map(PathBuf::from);
        let builtins_map_original_names = matches.is_present("builtins_map_original_names");
        let builtins_additional = matches
            .values_of("builtins_additional")
            .unwrap_or_default()
            .map(|name| name.to_string())
            .collect();
        let config = Config {
            input_path,
            output_path,
            patcher_config: PatcherConfig {
                builtins_path,
                builtins_map_path,
                builtins_map_original_names,
                builtins_additional,
            },
        };
        Ok(config)
    }
}
