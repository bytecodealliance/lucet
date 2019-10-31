#[macro_use]
extern crate clap;

use clap::Arg;
use failure::{format_err, Error};
use lucet_spectest;
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    let _ = include_str!("../Cargo.toml");
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("input")
                .multiple(false)
                .required(true)
                .help("input spec (.wast)"),
        )
        .get_matches();
    let input = matches.value_of("input").unwrap();
    let run = lucet_spectest::run_spec_test(&PathBuf::from(input))?;

    run.report();

    if run.failed().len() > 0 {
        Err(format_err!("{} failures", run.failed().len()))
    } else {
        Ok(())
    }
}
