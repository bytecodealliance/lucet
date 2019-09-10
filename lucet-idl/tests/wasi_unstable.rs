use lucet_idl;
use std::path::Path;

#[test]
fn validate_wasi_unstable() {
    lucet_idl::load_witx(Path::new("tests/wasi_unstable.witx"))
        .expect("parse and validate wasi_unstable.witx");
}
