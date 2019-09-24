#[cfg(test)]
mod wasi_sdk_tests {
    use lucet_validate::ModuleType;
    use std::path::PathBuf;

    fn wasi_sdk_test_source_file(name: &str) -> PathBuf {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("..");
        p.push("lucet-wasi-sdk");
        p.push("tests");
        p.push(name);
        assert!(p.exists(), "test file does not exist");
        p
    }

    fn compile_to_wasm(filename: &str) -> Vec<u8> {
        use lucet_wasi_sdk::{CompileOpts, Link, LinkOpt, LinkOpts};
        use std::fs::File;
        use std::io::Read;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("create temporary directory");

        let mut linker = Link::new(&[wasi_sdk_test_source_file(filename)]);
        linker.cflag("-nostartfiles");
        linker.link_opt(LinkOpt::NoDefaultEntryPoint);

        let wasmfile = tmp.path().join("out.wasm");

        linker.link(wasmfile.clone()).expect("link out.wasm");

        let mut module_contents = Vec::new();
        let mut file = File::open(wasmfile).expect("open out.wasm");
        file.read_to_end(&mut module_contents)
            .expect("read out.wasm");

        module_contents
    }

    #[test]
    fn moduletype_of_compiled() {
        let main_wasm = compile_to_wasm("main_returns.c");
        let moduletype = ModuleType::parse_wasm(&main_wasm).expect("main_returns has module type");
    }
}
