use clap::{App, Arg};
use failure::{format_err, Error};
use lucet_spectest;
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    let matches = App::new("lucet-spectest")
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
