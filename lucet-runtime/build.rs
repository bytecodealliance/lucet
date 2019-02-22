use cbindgen;
use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    cbindgen::Builder::new()
        .with_config(cbindgen::Config::from_root_or_default(&crate_dir))
        .with_crate(&crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include("lucet_val.h")
        .with_include("lucet_vmctx.h")
        .with_sys_include("signal.h")
        .with_sys_include("ucontext.h")
        .with_include_guard("LUCET_H")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("include/lucet.h");
}
