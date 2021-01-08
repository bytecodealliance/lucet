mod options;

#[macro_use]
extern crate clap;

use crate::options::{CodegenOutput, ErrorStyle, Options};
use anyhow::{format_err, Error};
use log::info;
use lucet_module::bindings::Bindings;
use lucetc::{
    signature::{self, PublicKey},
    Lucetc, LucetcOpts, Validator, WasiMode,
};
use serde::Serialize;

use std::path::PathBuf;
use std::process;

#[derive(Clone, Debug, Serialize)]
pub struct SerializedLucetcError {
    error: String,
}

impl From<Error> for SerializedLucetcError {
    fn from(e: Error) -> Self {
        SerializedLucetcError {
            error: format!("{}", e),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum BindingError {
    #[error("adding bindings from {1}")]
    ExtendError(#[source] lucet_module::error::Error, String),
    #[error("bindings file {1}")]
    FileError(#[source] lucet_module::error::Error, String),
}

fn main() {
    env_logger::init();

    let opts = Options::get().unwrap();

    if let Err(err) = run(&opts) {
        match opts.error_style {
            ErrorStyle::Human => {
                eprintln!("Error: {}\n", err);
            }
            ErrorStyle::Json => {
                let errs: Vec<SerializedLucetcError> = vec![err.into()];
                let json = serde_json::to_string(&errs).unwrap();
                eprintln!("{}", json);
            }
        }
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

    let mut validator = if !opts.witx_specs.is_empty() {
        Some(
            Validator::builder()
                .load_witx(&opts.witx_specs)?
                .wasi_mode(if opts.wasi_exe {
                    Some(WasiMode::Command)
                } else {
                    None
                })
                .build(),
        )
    } else {
        None
    };

    let mut bindings = Bindings::empty();
    if opts.wiggle_bindings {
        if let Some(ref v) = validator {
            for d in v.docs().iter() {
                bindings.extend(&lucet_wiggle_generate::bindings(d))?;
            }
        }
    }
    for file in opts.binding_files.iter() {
        let file_bindings = Bindings::from_file(file).map_err(|source| {
            let file = format!("{:?}", file);
            BindingError::FileError(source, file)
        })?;

        bindings.extend(&file_bindings).map_err(|source| {
            let file = format!("{:?}", file);
            BindingError::ExtendError(source, file)
        })?;
    }

    let mut c = Lucetc::new(PathBuf::from(input))
        .with_bindings(bindings)
        .with_opt_level(opts.opt_level)
        .with_cpu_features(opts.cpu_features.clone())
        .with_target(opts.target.clone())
        .with_target_version(opts.target_version.clone());

    if let Some(validator) = validator.take() {
        c.validator(validator);
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

    c.translate_wat(opts.translate_wat);

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
        _ => return Err(format_err!("Keypair generation requires --signature-pk and --signature-sk to specify where the keys should be stored to"))
    };
    signature::keygen(pk_path, sk_path)?;
    Ok(())
}
