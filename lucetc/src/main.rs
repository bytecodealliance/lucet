mod options;

use crate::options::{CodegenOutput, Options};
use failure::{format_err, Error, ResultExt};
use log::info;
use lucetc::{Bindings, Lucetc};

use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

fn main() {
    env_logger::init();

    let opts = Options::get().unwrap();

    if let Err(err) = run(&opts) {
        let mut msg = format!("{:?}", err);
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        io::stderr().write(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}

pub fn run(opts: &Options) -> Result<(), Error> {
    info!("lucetc {:?}", opts);

    let input = &match opts.input.len() {
        0 => Err(format_err!("must provide at least one input")),
        1 => Ok(opts.input[0].clone()),
        _ => Err(format_err!("provided too many inputs: {:?}", opts.input)),
    }?;

    let mut bindings = Bindings::empty();
    for file in opts.binding_files.iter() {
        let file_bindings =
            Bindings::from_file(file).context(format!("bindings file {:?}", file))?;
        bindings
            .extend(file_bindings)
            .context(format!("adding bindings from {:?}", file))?;
    }

    let mut c = Lucetc::new(PathBuf::from(input))?
        .bindings(bindings)?
        .opt_level(opts.opt_level);

    if let Some(ref builtins) = opts.builtins_path {
        c.with_builtins(builtins)?;
    }

    if let Some(min_reserved_size) = opts.min_reserved_size {
        c.with_min_reserved_size(min_reserved_size);
    }

    if let Some(max_reserved_size) = opts.max_reserved_size {
        c.with_max_reserved_size(max_reserved_size);
    }

    if let Some(guard_size) = opts.guard_size {
        c.with_guard_size(guard_size);
    }

    match opts.codegen {
        CodegenOutput::Obj => c.object_file(&opts.output)?,
        CodegenOutput::SharedObj => c.shared_object_file(&opts.output)?,
        CodegenOutput::Clif => c.clif_ir(&opts.output)?,
    }
    Ok(())
}
