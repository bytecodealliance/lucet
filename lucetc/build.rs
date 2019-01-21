use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

fn main() {
    let cargo_root = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("cargo provides CARGO_MANIFEST_DIR env var"),
    );

    let bindings_path = cargo_root
        .join("..")
        .join("lucet-libc")
        .join("src")
        .join("bindings.json");
    let mut contents = String::new();

    {
        let mut input = File::open(bindings_path).expect("bindings found in lucet-libc source");
        input
            .read_to_string(&mut contents)
            .expect("reading bindings");
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("paths.rs");
    let mut out = File::create(&dest_path).unwrap();

    out.write_all(
        format!(
            "pub const LUCET_LIBC_BINDINGS: &'static str = {:?};\n",
            contents,
        )
        .as_bytes(),
    )
    .unwrap();
}
