#[cfg(test)]
mod compile_and_link_tests {
    use lucet_wasi_sdk::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_file(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests");
        p.push(name);
        assert!(p.exists(), "test file does not exist");
        p
    }

    #[test]
    fn compile_a() {
        let tmp = TempDir::new().expect("create temporary directory");

        let compiler = Compile::new(test_file("a.c"));

        let objfile = tmp.path().join("a.o");

        compiler.compile(objfile.clone()).expect("compile a.c");

        assert!(objfile.exists(), "object file created");

        let mut linker = Link::new(&[objfile]);
        linker.cflag("-nostartfiles");
        linker.link_opt(LinkOpt::NoDefaultEntryPoint);

        let wasmfile = tmp.path().join("a.wasm");

        linker.link(wasmfile.clone()).expect("link a.wasm");

        assert!(wasmfile.exists(), "wasm file created");
    }

    #[test]
    fn compile_b() {
        let tmp = TempDir::new().expect("create temporary directory");

        let compiler = Compile::new(test_file("b.c"));

        let objfile = tmp.path().join("b.o");

        compiler.compile(objfile.clone()).expect("compile b.c");

        assert!(objfile.exists(), "object file created");

        let mut linker = Link::new(&[objfile]);
        linker.cflag("-nostartfiles");
        linker.link_opt(LinkOpt::NoDefaultEntryPoint);
        linker.link_opt(LinkOpt::AllowUndefinedAll);

        let wasmfile = tmp.path().join("b.wasm");

        linker.link(wasmfile.clone()).expect("link b.wasm");

        assert!(wasmfile.exists(), "wasm file created");
    }

    #[test]
    fn compile_a_and_b() {
        let tmp = TempDir::new().expect("create temporary directory");

        let mut linker = Link::new(&[test_file("a.c"), test_file("b.c")]);
        linker.cflag("-nostartfiles");
        linker.link_opt(LinkOpt::NoDefaultEntryPoint);

        let wasmfile = tmp.path().join("ab.wasm");

        linker.link(wasmfile.clone()).expect("link ab.wasm");

        assert!(wasmfile.exists(), "wasm file created");
    }

    #[test]
    fn compile_to_lucet() {
        let tmp = TempDir::new().expect("create temporary directory");

        let mut lucetc = Lucetc::new(&[test_file("a.c"), test_file("b.c")]);
        lucetc.cflag("-nostartfiles");
        lucetc.link_opt(LinkOpt::NoDefaultEntryPoint);

        let so_file = tmp.path().join("ab.so");

        lucetc.build(&so_file).expect("compile ab.so");

        assert!(so_file.exists(), "so file created");
    }
}
