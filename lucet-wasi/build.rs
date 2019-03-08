use std::env;
use std::path::PathBuf;

fn main() {
    let host_builder = bindgen::Builder::default()
        .clang_arg("-nostdinc")
        .clang_arg("-I/opt/fst-clang/7.0.0/lib/clang/7.0.0/include")
        .clang_arg("-I/usr/lib/clang/7/include")
        .header("wasi/include/wasi.h")
        .whitelist_type("__wasi_.*")
        .whitelist_var("__WASI_.*");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    host_builder
        .generate()
        .expect("can generate host bindings")
        .write_to_file(out_path.join("wasi_host.rs"))
        .expect("can write host bindings");

    // let guest_builder = bindgen::Builder::default()
    //     .clang_arg("--target=wasm32")
    //     .clang_arg("--sysroot=sysroot")
    //     .header("wasi/include/wasi.h");

    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    // guest_builder
    //     .generate()
    //     .expect("can generate guest bindings")
    //     .write_to_file(out_path.join("wasi_guest.rs"))
    //     .expect("can write guest bindings");
}
