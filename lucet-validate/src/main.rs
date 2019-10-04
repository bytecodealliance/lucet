#[macro_use]
extern crate clap;
use clap::Arg;
use failure::Fail;
use lucet_validate::{self, Validator};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;
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

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "{}", _0)]
    Witx(#[cause] witx::WitxError),
    #[fail(display = "With file {:?}: {}", _0, _1)]
    Io(PathBuf, #[cause] io::Error),
    #[fail(display = "{}", _0)]
    Validate(#[cause] lucet_validate::Error),
}

impl From<witx::WitxError> for Error {
    fn from(e: witx::WitxError) -> Error {
        Error::Witx(e)
    }
}

impl From<lucet_validate::Error> for Error {
    fn from(e: lucet_validate::Error) -> Error {
        Error::Validate(e)
    }
}
