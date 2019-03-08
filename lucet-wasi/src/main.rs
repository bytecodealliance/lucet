#[macro_use]
extern crate clap;

use clap::Arg;
use lucet_runtime::{DlModule, Limits, MmapRegion, Module, Region};
use lucet_wasi::hostcalls::WasiCtx;
use std::os::raw::c_void;
use std::sync::Arc;

struct Config {
    lucet_module: String,
}

fn main() {
    let matches = app_from_crate!()
        .arg(
            Arg::with_name("lucet_module")
                .required(true)
                .help("Path to the `lucetc`-compiled WASI module"),
        )
        .get_matches();
    let lucet_module = matches.value_of("lucet_module").unwrap().to_string();
    let config = Config { lucet_module };
    run(&config)
}

fn run(config: &Config) {
    let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");
    let module = DlModule::load(&config.lucet_module).expect("module can be loaded");
    let ctx = Box::new(WasiCtx::new());
    let mut inst = region
        .new_instance_with_ctx(module as Arc<dyn Module>, Box::into_raw(ctx) as *mut c_void)
        .expect("instance can be created");
    inst.run(b"_start", &[]).expect("instance runs");
}
