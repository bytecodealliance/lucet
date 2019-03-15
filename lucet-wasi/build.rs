use std::env;
use std::path::PathBuf;

fn main() {
    let wasi_sdk = env::var("WASI_SDK").unwrap_or("/opt/wasi-sdk".to_owned());

    let host_builder = bindgen::Builder::default()
        .clang_arg("-nostdinc")
        .clang_arg("-D__wasi__")
        .clang_arg(format!("-isystem={}/share/sysroot/include/", wasi_sdk))
        .clang_arg(format!("-I{}/lib/clang/8.0.0/include/", wasi_sdk))
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
