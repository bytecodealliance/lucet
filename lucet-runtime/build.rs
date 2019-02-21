use bindgen;
use cbindgen;
use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    cbindgen::Builder::new()
        .with_config(cbindgen::Config::from_root_or_default(&crate_dir))
        .with_crate(&crate_dir)
        .with_language(cbindgen::Language::C)
        .with_parse_deps(true)
        // .with_parse_include(&["lucet-runtime-internals"])
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("lucet.h");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindgen::Builder::default()
        .clang_arg("-std=gnu99")
        .header(
            crate_dir
                .join("include")
                .join("lucet_val.h")
                .to_str()
                .unwrap(),
        )
        .whitelist_type("lucet_.*")
        .whitelist_var("lucet_.*")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("lucet_val.rs"))
        .expect("Couldn't write bindings!");

    // bindgen::Builder::default()
    //     .clang_arg("-std=gnu99")
    //     .header(
    //         crate_dir
    //             .join("include")
    //             .join("lucet_state.h")
    //             .to_str()
    //             .unwrap(),
    //     )
    //     .whitelist_type("lucet_.*")
    //     .whitelist_var("lucet_.*")
    //     .generate()
    //     .expect("Unable to generate bindings")
    //     .write_to_file(out_path.join("lucet_state.rs"))
    //     .expect("Couldn't write bindings!");
}
