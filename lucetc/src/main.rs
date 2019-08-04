mod options;

use crate::options::{CodegenOutput, Options};
use failure::{format_err, Error, ResultExt};
use log::info;
use lucet_module::bindings::Bindings;
use lucetc::{
    signature::{self, PublicKey},
    Lucetc, LucetcOpts,
};
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
        io::stderr().write_all(msg.as_bytes()).unwrap();
        process::exit(1);
    }
}

pub fn run(opts: &Options) -> Result<(), Error> {
    info!("lucetc {:?}", opts);

    if opts.keygen {
        keygen(opts)?;
        return Ok(());
    }

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
            .extend(&file_bindings)
            .context(format!("adding bindings from {:?}", file))?;
    }

    let mut c = Lucetc::new(PathBuf::from(input))
        .with_bindings(bindings)
        .with_opt_level(opts.opt_level);

    if let Some(ref builtins) = opts.builtins_path {
        c.builtins(builtins);
    }

    if let Some(min_reserved_size) = opts.min_reserved_size {
        c.min_reserved_size(min_reserved_size);
    }

    if let Some(max_reserved_size) = opts.max_reserved_size {
        c.max_reserved_size(max_reserved_size);
    }

    // this comes after min and max, so it overrides them if present
    if let Some(reserved_size) = opts.reserved_size {
        c.reserved_size(reserved_size);
    }

    if let Some(guard_size) = opts.guard_size {
        c.guard_size(guard_size);
    }

    if let Some(pk_path) = &opts.pk_path {
        c.pk(PublicKey::from_file(pk_path)?);
    }

    if let Some(sk_path) = &opts.sk_path {
        c.sk(signature::sk_from_file(sk_path)?);
    }

    if opts.verify {
        c.verify();
    }

    if opts.sign {
        c.sign();
    }

    if opts.count_instructions {
        c.count_instructions(true);
    }

    match opts.codegen {
        CodegenOutput::Obj => c.object_file(&opts.output)?,
        CodegenOutput::SharedObj => c.shared_object_file(&opts.output)?,
        CodegenOutput::Clif => c.clif_ir(&opts.output)?,
    }
    Ok(())
}

fn keygen(opts: &Options) -> Result<(), Error> {
    let (pk_path, sk_path) = match (&opts.pk_path, &opts.sk_path) {
        (Some(pk_path), Some(sk_path)) => (pk_path, sk_path),
        _ => Err(format_err!("Keypair generation requires --signature-pk and --signature-sk to specify where the keys should be stored to"))?
    };
    signature::keygen(pk_path, sk_path)?;
    Ok(())
}
