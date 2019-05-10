use lucet_idl::parse_package;
use lucet_idl_test::syntax::{DatatypeSyntax, Spec};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

fn main() {
    let mut runner = TestRunner::default();
    let dts = prop::collection::vec(DatatypeSyntax::strat(), 0..20)
        .new_tree(&mut runner)
        .unwrap()
        .current();
    let spec = Spec::from_decls(dts);
    let rendered = spec.render_idl();
    println!("{}", rendered);

    let _pkg = parse_package(&rendered).expect("parse generated package");
}
