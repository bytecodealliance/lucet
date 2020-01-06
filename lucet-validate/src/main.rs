#[macro_use]
extern crate clap;
use clap::Arg;
use lucet_validate::{self, Validator};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;
use thiserror::Error;
use witx;

pub fn main() {
    // rebuild if env vars used by app_from_crate! change:
    let _ = include_str!("../Cargo.toml");

    let matches = app_from_crate!()
        .arg(
            Arg::with_name("module")
                .takes_value(true)
                .required(true)
                .help("WebAssembly module"),
        )
        .arg(
            Arg::with_name("witx")
                .takes_value(true)
                .required(true)
                .help("validate against interface in this witx file"),
        )
        .arg(
            Arg::with_name("wasi-exe")
                .takes_value(false)
                .required(false)
                .short("w")
                .long("wasi-exe")
                .help("validate exports of WASI executable"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .takes_value(false)
                .required(false),
        )
        .get_matches();

    let module_path = matches
        .value_of("module")
        .map(Path::new)
        .expect("module arg required");

    match run(
        &module_path,
        Path::new(matches.value_of("witx").expect("witx path required")),
        matches.is_present("wasi-exe"),
    ) {
        Ok(()) => {
            if matches.is_present("verbose") {
                println!("validated successfully")
            }
        }
        Err(e) => {
            if matches.is_present("verbose") {
                match e {
                    Error::Witx(e) => {
                        println!("{}", e.report());
                    }
                    _ => {
                        println!("{:?}", e);
                    }
                }
            } else {
                println!("{}", e);
            }
            process::exit(-1);
        }
    }
}

fn run(module_path: &Path, witx_path: &Path, wasi_exe: bool) -> Result<(), Error> {
    let mut module_contents = Vec::new();
    let mut file = File::open(module_path).map_err(|e| Error::Io(module_path.into(), e))?;
    file.read_to_end(&mut module_contents)
        .map_err(|e| Error::Io(module_path.into(), e))?;

    let validator = Validator::load(witx_path)?.with_wasi_exe(wasi_exe);
    validator.validate(&module_contents)?;

    Ok(())
}

#[derive(Debug, Error)]
enum Error {
    #[error("Witx error")]
    Witx(#[from] witx::WitxError),
    #[error("With file {0:?}: {1}")]
    Io(PathBuf, #[source] io::Error),
    #[error("Validate error")]
    Validate(#[from] lucet_validate::Error),
}
