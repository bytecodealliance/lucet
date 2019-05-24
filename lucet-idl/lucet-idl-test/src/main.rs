use env_logger;
use log::{debug, info};
use lucet_idl::parse_package;
use lucet_idl_test::{CGuestApp, HostApp, RustGuestApp, Spec};
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

    let mut rust_guest_app = RustGuestApp::new().expect("create rust guest app");
    let rust_guest_so = rust_guest_app.build(&pkg).expect("compile rust guest app");

    let mut c_guest_app = CGuestApp::new().expect("create c guest app");
    let c_guest_so = c_guest_app.build(&pkg).expect("compile c guest app");

    let mut host_app = HostApp::new(&pkg).expect("create host app");
    host_app.run(&rust_guest_so).expect("run rust_guest_so");
    host_app.run(&c_guest_so).expect("run c_guest_so");
}
