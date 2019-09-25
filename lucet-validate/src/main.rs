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
        .arg(Arg::with_name("module").takes_value(true).required(true))
        .arg(
            Arg::with_name("witx")
                .takes_value(true)
                .required(false)
                .long("witx")
                .help("validate against interface in this witx file"),
        )
        .arg(
            Arg::with_name("wasi")
                .takes_value(false)
                .required(false)
                .long("wasi")
                .help("validate against wasi interface"),
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
        matches.value_of("witx"),
        matches.is_present("wasi"),
    ) {
        Ok(()) => {}
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

fn run(module_path: &Path, witx_path: Option<&str>, wasi_spec: bool) -> Result<(), Error> {
    let interface_doc = match (witx_path, wasi_spec) {
        (Some(witx_path), false) => witx::load(Path::new(witx_path))?,
        (None, true) => wasi::unstable::preview0(),
        (Some(_), true) => Err(Error::Usage(
            "Cannot validate against both witx and wasi spec",
        ))?,
        (None, false) => Err(Error::Usage("must provide at least one spec"))?,
    };

    let mut module_contents = Vec::new();
    let mut file = File::open(module_path).map_err(|e| Error::Io(module_path.into(), e))?;
    file.read_to_end(&mut module_contents)
        .map_err(|e| Error::Io(module_path.into(), e))?;

    lucet_validate::validate(&interface_doc, &module_contents)?;

    Ok(())
}

#[derive(Debug, Fail)]
enum Error {
    #[fail(display = "Usage error: {}", _0)]
    Usage(&'static str),
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
