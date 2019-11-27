#[cfg(test)]
mod lucet_validate_tests {
    use lucet_validate::Validator;
    use std::fs;
    use std::path::Path;
    use wabt;

    fn c_to_wasm(c_path: &Path) -> Vec<u8> {
        use lucet_wasi_sdk::Link;
        use std::fs::File;
        use std::io::Read;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("create temporary directory");

        let wasmfile = tmp.path().join("out.wasm");
        let linker = Link::new(&[c_path]);
        linker.link(wasmfile.clone()).expect("link out.wasm");

        let mut module_contents = Vec::new();
        let mut file = File::open(wasmfile).expect("open out.wasm");
        file.read_to_end(&mut module_contents)
            .expect("read out.wasm");

        module_contents
    }

    fn wat_to_wasm(wat_path: &Path) -> Vec<u8> {
        use std::fs::File;
        use std::io::Read;

        let mut wat_contents = Vec::new();
        let mut file = File::open(wat_path).expect("open wat");
        file.read_to_end(&mut wat_contents).expect("read wat");
        wabt::wat2wasm(wat_contents).expect("wat2wasm")
    }

    #[test]
    fn validate_lucet_wasi_test_guests() {
        let validator = Validator::load("../wasi/phases/old/snapshot_0/witx/wasi_unstable.witx")
            .expect("load wasi_unstable_preview0");

        for entry in
            fs::read_dir("../lucet-wasi/tests/guests").expect("read lucet_wasi test guests dir")
        {
            let entry_path = entry.expect("file from lucet_wasi test guests dir").path();
            let entry_wasm = match entry_path
                .extension()
                .map(|s| s.to_str().expect("extension is str"))
            {
                Some("c") => c_to_wasm(&entry_path),
                Some("wat") => wat_to_wasm(&entry_path),
                _ => {
                    eprintln!("unsupported extension: {:?}", entry_path);
                    continue;
                }
            };
            validator
                .validate(&entry_wasm)
                .expect(&format!("validate {:?}", entry_path));
        }
    }
}
