#[cfg(test)]
mod wasi_sdk_tests {
    use lucet_validate::{self, ModuleType};
    use std::path::PathBuf;
    use witx;

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
        use lucet_wasi_sdk::Link;
        use std::fs::File;
        use std::io::Read;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("create temporary directory");

        let wasmfile = tmp.path().join("out.wasm");
        let linker = Link::new(&[wasi_sdk_test_source_file(filename)]);
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
        let _moduletype = ModuleType::parse_wasm(&main_wasm).expect("main_returns has module type");
    }

    #[test]
    fn validate_compiled() {
        let main_wasm = compile_to_wasm("main_returns.c");
        let wasi_spec = witx::load("../wasi/phases/unstable/witx/wasi_unstable_preview0.witx")
            .expect("load wasi_unstable_preview0");
        lucet_validate::validate(&wasi_spec, &main_wasm, true).expect("validation");
    }
}
