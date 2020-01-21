#[cfg(test)]
mod lucetc_tests {
    use anyhow::Error;
    use lucet_module::bindings::Bindings;
    use lucet_validate::Validator;
    use lucet_wasi_sdk::*;
    use lucetc::{Compiler, CpuFeatures, HeapSettings, OptLevel};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;
    use target_lexicon::Triple;

    /// Compile C -> WebAssembly using wasi-sdk's clang. Does not use the wasi-sdk
    /// libc, and does not produce a wasi executable, just a wasm module with the given set of
    /// export functions.
    fn module_from_c(cfiles: &[&str], exports: &[&str]) -> Result<Vec<u8>, Error> {
        let cfiles: Vec<PathBuf> = cfiles
            .iter()
            .map(|ref name| PathBuf::from(format!("tests/{}.c", name)))
            .collect();
        let tempdir = tempfile::Builder::new().prefix("wasi-sdk-test").tempdir()?;

        let mut wasm = PathBuf::from(tempdir.path());
        wasm.push("out.wasm");

        let mut linker = Link::new(&cfiles)
            .with_cflag("-nostartfiles")
            .with_link_opt(LinkOpt::NoDefaultEntryPoint)
            .with_link_opt(LinkOpt::AllowUndefinedAll);
        for export in exports {
            linker.export(export);
        }
        linker.link(wasm.clone())?;

        let mut wasm_file = File::open(wasm)?;
        let mut wasm_contents = Vec::new();
        wasm_file.read_to_end(&mut wasm_contents)?;
        Ok(wasm_contents)
    }

    #[test]
    fn empty() {
        let m = module_from_c(&["empty"], &[]).expect("build module for empty");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let v = Validator::parse("").expect("empty validation environment");
        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &Some(v),
        )
        .expect("compile empty");
        let mdata = c.module_data().unwrap();
        assert!(mdata.heap_spec().is_some());
        // clang creates just 1 global:
        assert_eq!(mdata.globals_spec().len(), 1);
        assert!(mdata.globals_spec()[0].is_internal());

        assert_eq!(mdata.import_functions().len(), 0, "import functions");
        assert_eq!(mdata.export_functions().len(), 0, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */

        let _obj = c.object_file().expect("generate code from empty");
    }

    fn d_only_test_bindings() -> Bindings {
        let imports: HashMap<String, String> = [
            ("c".into(), "c".into()), // d_only
        ]
        .iter()
        .cloned()
        .collect();

        Bindings::env(imports)
    }

    #[test]
    fn just_c() {
        let m = module_from_c(&["c"], &["c"]).expect("build module for c");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let v = Validator::parse("").expect("empty validation environment");

        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &Some(v),
        )
        .expect("compile c");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.import_functions().len(), 0, "import functions");
        assert_eq!(mdata.export_functions().len(), 1, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */

        let _obj = c.object_file().expect("generate code from c");
    }

    #[test]
    fn just_d() {
        let m = module_from_c(&["d"], &["d"]).expect("build module for d");
        let b = d_only_test_bindings();
        let h = HeapSettings::default();
        let v = Validator::parse(
            "(module $env (@interface func (export \"c\") (param $a1 s32) (result $r1 s32)))",
        )
        .expect("empty validation environment");
        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &Some(v),
        )
        .expect("compile d");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.import_functions().len(), 1, "import functions");
        assert_eq!(mdata.export_functions().len(), 1, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */
        let _obj = c.object_file().expect("generate code from d");
    }

    #[test]
    fn c_and_d() {
        let m = module_from_c(&["c", "d"], &["c", "d"]).expect("build module for c & d");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let v = Validator::parse("").expect("empty validation environment");
        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &Some(v),
        )
        .expect("compile c & d");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.import_functions().len(), 0, "import functions");
        assert_eq!(mdata.export_functions().len(), 2, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */
        let _obj = c.object_file().expect("generate code from c & d");
    }

    #[test]
    fn hello() {
        let m = {
            // Unlike in module_from_c, use wasi-sdk to compile a C file to a wasi executable,
            // linking in wasi-libc and exposing the wasi _start entry point only:
            let tempdir = tempfile::Builder::new()
                .prefix("wasi-sdk-test")
                .tempdir()
                .expect("create tempdir");
            let mut wasm = PathBuf::from(tempdir.path());
            wasm.push("out.wasm");

            let linker = Link::new(&[PathBuf::from("tests/hello.c")]);
            linker.link(wasm.clone()).expect("link");

            let mut wasm_file = File::open(wasm).expect("open wasm");
            let mut wasm_contents = Vec::new();
            wasm_file
                .read_to_end(&mut wasm_contents)
                .expect("read wasm");
            wasm_contents
        };

        let b =
            Bindings::from_file("../lucet-wasi/bindings.json").expect("load lucet-wasi bindings");
        let h = HeapSettings::default();
        let v = Validator::load("../wasi/phases/old/snapshot_0/witx/wasi_unstable.witx")
            .expect("wasi spec validation")
            .with_wasi_exe(true);
        // Compiler will only unwrap if the Validator defined above accepts the module
        let c = Compiler::new(
            &m,
            Triple::host(),
            OptLevel::default(),
            CpuFeatures::default(),
            &b,
            h,
            false,
            &Some(v),
        )
        .expect("compile empty");
        let mdata = c.module_data().unwrap();
        assert!(mdata.heap_spec().is_some());
    }
}
