use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    let wasi_sdk = env::var("WASI_SDK").unwrap_or("/opt/wasi-sdk".to_owned());

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let core_h_path = out_path.join("core.h");
    let core_h = File::create(&core_h_path).unwrap();

    // `bindgen` doesn't understand typed constant macros like `UINT8_C(123)`, so this fun regex
    // strips them off to yield a copy of `wasi/core.h` with bare constants.
    Command::new("sed")
        .arg("-E")
        .arg(r#"s/U?INT[0-9]+_C\(((0x)?[0-9]+)\)/\1/g"#)
        .arg("/opt/wasi-sdk/share/sysroot/include/wasi/core.h")
        .stdout(Stdio::from(core_h))
        .status()
        .unwrap();

    let host_builder = bindgen::Builder::default()
        .clang_arg("-nostdinc")
        .clang_arg("-D__wasi__")
        .clang_arg(format!("-isystem={}/share/sysroot/include/", wasi_sdk))
        .clang_arg(format!("-I{}/lib/clang/8.0.0/include/", wasi_sdk))
        .header(core_h_path.to_str().unwrap())
        .whitelist_type("__wasi_.*")
        .whitelist_var("__WASI_.*");

    host_builder
        .generate()
        .expect("can generate host bindings")
        .write_to_file(out_path.join("wasi_host.rs"))
        .expect("can write host bindings");
}
