extern crate capnpc;

fn main() {
    ::capnpc::CompilerCommand::new()
        .file("lucet-module-data.capnp")
        .edition(::capnpc::RustEdition::Rust2018)
        .run()
        .expect("compiling schema");
}
