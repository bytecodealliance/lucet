use bindgen;
use std::env;
use std::path::PathBuf;

fn main() {
    let mut liblucet_runtime_c_base_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo env var set"));
    liblucet_runtime_c_base_dir.push("..");
    liblucet_runtime_c_base_dir.push("..");
    liblucet_runtime_c_base_dir.push("lucet-runtime-c");

    let liblucet_runtime_c_install_dir = PathBuf::from("/opt/fst-liblucet-runtime-c/lib/");

    let mut liblucet_runtime_c_build_dir = liblucet_runtime_c_base_dir.clone();
    liblucet_runtime_c_build_dir.push("build");
    if liblucet_runtime_c_build_dir.exists() {
        let liblucet_runtime_c_build_dir =
            std::fs::canonicalize(&liblucet_runtime_c_build_dir).expect("absolute path");
        println!(
            "cargo:rustc-link-search=native={}",
            liblucet_runtime_c_build_dir.to_str().expect("path")
        );
    } else if liblucet_runtime_c_install_dir.exists() {
        println!("cargo:rustc-link-search=native=/opt/fst-liblucet-runtime-c/lib/");
    } else {
        panic!("cannot link lucet-sys: liblucet-runtime-c needs to either be built in its source tree or installed in /opt/fst-liblucet-runtime-c/lib!")
    }

    println!("cargo:rustc-link-lib=dylib=lucet-runtime-c");

    let mut liblucet_runtime_c_include_dir = liblucet_runtime_c_base_dir.clone();
    liblucet_runtime_c_include_dir.push("include");

    let mut liblucet_runtime_c_h = liblucet_runtime_c_include_dir.clone();
    liblucet_runtime_c_h.push("lucet.h");

    let bindings = bindgen::Builder::default()
        .clang_arg("-std=gnu99")
        .clang_arg("-D_GNU_SOURCE")
        .header(liblucet_runtime_c_h.to_str().expect("header"))
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
        .rustfmt_bindings(true)
        .time_phases(true);

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

    let mut liblucet_runtime_c_src_dir = liblucet_runtime_c_base_dir.clone();
    liblucet_runtime_c_src_dir.push("src");

    let mut instance_private_h = liblucet_runtime_c_src_dir.clone();
    instance_private_h.push("lucet_instance_private.h");

    let mut alloc_private_h = liblucet_runtime_c_src_dir.clone();
    alloc_private_h.push("lucet_alloc_private.h");

    let internal_bindings = bindgen::Builder::default()
        .clang_arg("-std=gnu99")
        .clang_arg("-D_GNU_SOURCE")
        .clang_arg(format!(
            "-I{}",
            liblucet_runtime_c_include_dir
                .to_str()
                .expect("clang src dir"),
        ))
        .header(instance_private_h.to_str().expect("header"))
        .header(alloc_private_h.to_str().expect("header"))
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
        .rustfmt_bindings(true)
        .time_phases(true);

    let internal_bindings = if let Ok(libclang_include_dir) = env::var("LUCET_LIBCLANG_INCLUDE") {
        internal_bindings.clang_arg(format!("-I{}", libclang_include_dir))
    } else {
        internal_bindings
    };

    let internal_bindings = internal_bindings
        .generate()
        .expect("Unable to generate internal bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    internal_bindings
        .write_to_file(out_path.join("internal_bindings.rs"))
        .expect("Couldn't write internal bindings!");
}
