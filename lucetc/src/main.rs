mod options;

use crate::options::{CodegenOutput, Options, LUCET_LIBC_BINDINGS};
use failure::{format_err, Error, ResultExt};
use log::info;
use lucetc::bindings::Bindings;
use lucetc::load::read_module;
use lucetc::patch::patch_module;
use lucetc::program::Program;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{self, Command};

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

    let name = String::from(
        PathBuf::from(input)
            .file_stem()
            .ok_or(format_err!("input filename {:?} is not a file", input))?
            .to_str()
            .ok_or(format_err!("input filename {:?} is not valid utf8", input))?,
    );

    let module = read_module(&PathBuf::from(input)).context("loading module {}")?;

    let (module, builtins_map) = if let Some(builtins_path) = &opts.builtins_path {
        patch_module(module, builtins_path)?
    } else {
        (module, HashMap::new())
    };

    let mut bindings = if opts.liblucet_runtime_c_bindings {
        Bindings::from_str(LUCET_LIBC_BINDINGS).context("lucet_libc bindings")?
    } else {
        Bindings::empty()
    };

    bindings.extend(Bindings::env(builtins_map))?;

    for file in opts.binding_files.iter() {
        let file_bindings =
            Bindings::from_file(file).context(format!("bindings file {:?}", file))?;
        bindings
            .extend(file_bindings)
            .context(format!("adding bindings from {:?}", file))?;
    }

    let prog = Program::new(module, bindings, opts.heap.clone())?;
    let comp = lucetc::compile(&prog, &name, opts.opt_level)?;

    if opts.print_isa {
        println!("{}", comp.isa())
    }

    match opts.codegen {
        CodegenOutput::Obj => {
            let obj = comp.codegen()?;
            obj.write(&opts.output).context("writing object file")?;
        }
        CodegenOutput::SharedObj => {
            let dir = tempfile::Builder::new().prefix("lucetc").tempdir()?;
            let objpath = dir.path().join("tmp.o");

            let obj = comp.codegen()?;
            obj.write(&objpath).context("writing object file")?;

            let mut cmd_ld = Command::new("ld");
            cmd_ld.arg(objpath.clone());
            cmd_ld.arg("-shared");
            cmd_ld.arg("-o");
            cmd_ld.arg(opts.output.clone());

            let run_ld = cmd_ld
                .output()
                .context(format_err!("running ld on {:?}", objpath.clone()))?;

            if !run_ld.status.success() {
                Err(format_err!(
                    "ld of {} failed: {}",
                    objpath.to_str().unwrap(),
                    String::from_utf8_lossy(&run_ld.stderr)
                ))?;
            }
        }
        CodegenOutput::Clif => {
            comp.cranelift_funcs()
                .write(&opts.output)
                .context("writing clif file")?;
        }
    }
    Ok(())
}
