use env_logger;
use log::{debug, info};
use lucet_idl::parse_package;
use lucet_idl_test::syntax::Spec;
use lucet_idl_test::wasi::{WasiHostBuild, WasiProject};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

fn main() {
    env_logger::init();

    let mut runner = TestRunner::default();
    let spec = Spec::strat(10).new_tree(&mut runner).unwrap().current();
    let rendered = spec.render_idl();
    info!("generated spec:\n{}", rendered);

    let pkg = parse_package(&rendered).expect("parse generated package");

    debug!("parsed package: {:?}", pkg);
    let wasi_project = WasiProject::new(pkg);

    if false {
        let _rust_guest = wasi_project
            .codegen_rust_guest()
            .expect("compile rust guest");
        let _rust_host = wasi_project.compile_rust_host().expect("compile rust host");
    } else {
        let _wasi_host = WasiHostBuild::new().unwrap();
    }
}
