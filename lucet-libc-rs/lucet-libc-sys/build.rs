use bindgen;
use std::env;
use std::path::PathBuf;

fn main() {
    let mut lucet_libc_base_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo env var set"));
    lucet_libc_base_dir.push("..");
    lucet_libc_base_dir.push("..");
    lucet_libc_base_dir.push("lucet-libc");

    let lucet_libc_install_dir = PathBuf::from("/opt/fst-lucet-libc/lib/");

    let mut lucet_libc_build_dir = lucet_libc_base_dir.clone();
    lucet_libc_build_dir.push("build");
    lucet_libc_build_dir.push("lib");
    if lucet_libc_build_dir.exists() {
        let lucet_libc_build_dir =
            std::fs::canonicalize(&lucet_libc_build_dir).expect("absolute path");
        println!(
            "cargo:rustc-link-search=native={}",
            lucet_libc_build_dir.to_str().expect("path")
        );
    } else if lucet_libc_install_dir.exists() {
        println!("cargo:rustc-link-search=native=/opt/fst-lucet-libc/lib/");
    } else {
        panic!("cannot link lucet-libc-sys: lucet-libc needs to either be built in its source tree or installed in /opt/fst-lucet-libc/lib!")
    }

    println!("cargo:rustc-link-lib=static=lucet_libc");

    let mut lucet_libc_include_dir = lucet_libc_base_dir.clone();
    lucet_libc_include_dir.push("src");
    lucet_libc_include_dir.push("host");
    lucet_libc_include_dir.push("include");

    let mut lucet_libc_h = lucet_libc_include_dir.clone();
    lucet_libc_h.push("lucet_libc.h");

    let bindings = bindgen::Builder::default()
        .clang_arg("-std=gnu99")
        .clang_arg("-D_GNU_SOURCE")
        .header(lucet_libc_h.to_str().expect("header"))
        .whitelist_function("lucet_.*")
        .whitelist_type("lucet_.*")
        .whitelist_var("lucet_.*")
        .whitelist_var("LUCET_.*")
        .derive_copy(true)
        .derive_debug(true)
        .derive_default(true)
        .derive_eq(true)
        .generate_comments(true)
        .layout_tests(true)
        .prepend_enum_name(true)
        .rustfmt_bindings(true);

    let bindings = if let Ok(libclang_include_dir) = env::var("LUCET_LIBCLANG_INCLUDE") {
        bindings.clang_arg(format!("-I{}", libclang_include_dir))
    } else {
        bindings
    };

    let bindings = bindings.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
