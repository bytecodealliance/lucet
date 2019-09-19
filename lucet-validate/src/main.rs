#[macro_use]
extern crate clap;
use clap::Arg;
use failure::Fail;
use lucet_validate;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;
use witx;

pub fn main() {
    // rebuild if env vars used by app_from_crate! change:
    let _ = include_str!("../Cargo.toml");

    let matches = app_from_crate!()
        .arg(Arg::with_name("interface").takes_value(true).required(true))
        .arg(Arg::with_name("module").takes_value(true).required(true))
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .takes_value(false)
                .required(false),
        )
        .get_matches();

    let interface_path = PathBuf::from(
        matches
            .value_of("interface")
            .expect("interface arg required"),
    );
    let input_path = PathBuf::from(matches.value_of("module").expect("module arg required"));

    match run(&interface_path, &input_path) {
        Ok(()) => {}
        Err(e) => {
            if matches.is_present("verbose") {
                println!("{:?}", e);
            } else {
                println!("{}", e);
            }
            process::exit(-1);
        }
    }
}

fn run(interface: &Path, module: &Path) -> Result<(), Error> {
    let interface_doc = witx::load(interface)?;

    let mut module_contents = Vec::new();
    let mut file = File::open(module).map_err(|e| Error::Io(module.into(), e))?;
    file.read_to_end(&mut module_contents)
        .map_err(|e| Error::Io(module.into(), e))?;

    lucet_validate::validate(&interface_doc, &module_contents)?;

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
