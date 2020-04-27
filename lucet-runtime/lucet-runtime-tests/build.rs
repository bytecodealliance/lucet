use std::env;

fn main() {
    // TODO: this should only be built for tests, but Cargo doesn't
    // currently let you specify different build.rs options for tests:
    // <https://github.com/rust-lang/cargo/issues/1581>
    let traps_asm_file = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => "traps_x86_64.S",
        "x86" => "traps_i686.S",
        arch => {
            panic!("unsupported architecture {}", arch);
        }
    };

    cc::Build::new()
        .file(&format!("src/guest_fault/{}", traps_asm_file))
        .compile("context_context_asm");
}
