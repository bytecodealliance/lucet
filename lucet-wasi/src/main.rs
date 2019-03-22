#[macro_use]
extern crate clap;

use clap::Arg;
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::ctx::WasiCtx;
use std::sync::Arc;

struct Config<'a> {
    lucet_module: &'a str,
    guest_args: Vec<&'a str>,
}

fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("lucet_module")
                .required(true)
                .help("Path to the `lucetc`-compiled WASI module"),
        )
        .arg(
            Arg::with_name("guest_args")
                .required(false)
                .multiple(true)
                .help("Arguments to the WASI `main` function"),
        )
        .get_matches();
    let lucet_module = matches.value_of("lucet_module").unwrap();
    let guest_args = matches
        .values_of("guest_args")
        .map(|vals| vals.collect())
        .unwrap_or(vec![]);
    let config = Config {
        lucet_module,
        guest_args,
    };
    run(config)
}

fn run(config: Config) {
    let exitcode = {
        // doing all of this in a block makes sure everything gets dropped before exiting
        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");
        let module = DlModule::load(&config.lucet_module).expect("module can be loaded");

        // put the path to the module on the front for argv[0]
        let args = std::iter::once(config.lucet_module)
            .chain(config.guest_args.into_iter())
            .collect::<Vec<&str>>();
        let mut inst = region
            .new_instance_builder(module as Arc<dyn Module>)
            .with_embed_ctx(WasiCtx::new(&args))
            .build()
            .expect("instance can be created");

        match inst.run(b"_start", &[]) {
            // normal termination implies 0 exit code
            Ok(_) => 0,
            Err(lucet_runtime::Error::RuntimeTerminated(
                lucet_runtime::TerminationDetails::Provided(any),
            )) => *any
                .downcast_ref::<lucet_wasi::host::__wasi_exitcode_t>()
                .expect("termination yields an exitcode"),
            Err(e) => panic!("lucet-wasi runtime error: {}", e),
        }
    };
    std::process::exit(exitcode as i32);
}
