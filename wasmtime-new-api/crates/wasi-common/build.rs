// Tell any dependencies, if necessary, where our WASI submodule is so they can
// use the same witx files if they want.
fn main() {
    let cwd = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let wasi = cwd
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wasmtime")
        .join("crates")
        .join("wasi-common")
        .join("WASI");
    // this will be available to dependent crates via the DEP_WASI_COMMON_19_WASI env var:
    println!("cargo:wasi={}", wasi.display());
    // and available to our own crate as WASI_ROOT:
    println!("cargo:rustc-env=WASI_ROOT={}", wasi.display());
}
