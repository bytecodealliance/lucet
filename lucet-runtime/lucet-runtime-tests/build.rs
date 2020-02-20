fn main() {
    // TODO: this should only be built for tests, but Cargo doesn't
    // currently let you specify different build.rs options for tests:
    // <https://github.com/rust-lang/cargo/issues/1581>
    cc::Build::new()
        .file("src/guest_fault/traps.S")
        .compile("guest_fault_traps");
}
