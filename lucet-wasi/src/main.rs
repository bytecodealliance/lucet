#[macro_use]
extern crate clap;

use clap::Arg;
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::hostcalls::WasiCtx;
use std::sync::Arc;

struct Config<'a> {
    lucet_module: String,
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
    let lucet_module = matches.value_of("lucet_module").unwrap().to_string();
    let guest_args = matches
        .values_of("guest_args")
        .map(|vals| vals.collect())
        .unwrap_or(vec![]);
    let config = Config {
        lucet_module,
        guest_args,
    };
    run(&config)
}

fn run(config: &Config) {
    let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");
    let module = DlModule::load(&config.lucet_module).expect("module can be loaded");
    let mut inst = region
        .new_instance_builder(module as Arc<dyn Module>)
        .with_embed_ctx(WasiCtx::new(&config.lucet_module, &config.guest_args))
        .build()
        .expect("instance can be created");
    inst.run(b"_start", &[]).expect("instance runs");
}
