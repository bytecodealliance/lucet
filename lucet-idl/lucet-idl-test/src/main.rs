use lucet_idl::parse_package;
use lucet_idl_test::syntax::Spec;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

fn main() {
    let mut runner = TestRunner::default();
    let spec = Spec::strat(10).new_tree(&mut runner).unwrap().current();
    let rendered = spec.render_idl();
    //println!("{}", rendered);

    let pkg = parse_package(&rendered).expect("parse generated package");

    let _wasm = lucet_idl_test::compile::rust_wasm_codegen(&pkg);
}
