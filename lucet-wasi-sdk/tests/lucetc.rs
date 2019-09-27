#[cfg(test)]
mod lucetc_tests {
    use failure::Error;
    use lucet_module::bindings::Bindings;
    use lucet_wasi_sdk::*;
    use lucetc::{Compiler, HeapSettings, OptLevel};
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

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
        let c = Compiler::new(&m, OptLevel::default(), &b, h, false).expect("compile empty");
        let mdata = c.module_data().unwrap();
        assert!(mdata.heap_spec().is_some());
        // clang creates 3 globals:
        assert_eq!(mdata.globals_spec().len(), 3);
        assert!(mdata.globals_spec()[0].is_internal());
        assert_eq!(mdata.globals_spec()[1].export_names(), &["__heap_base"]);
        assert_eq!(mdata.globals_spec()[2].export_names(), &["__data_end"]);

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
        let c = Compiler::new(&m, OptLevel::default(), &b, h, false).expect("compile c");
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
        let c = Compiler::new(&m, OptLevel::default(), &b, h, false).expect("compile d");
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
        let c = Compiler::new(&m, OptLevel::default(), &b, h, false).expect("compile c & d");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.import_functions().len(), 0, "import functions");
        assert_eq!(mdata.export_functions().len(), 2, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */
        let _obj = c.object_file().expect("generate code from c & d");
    }
}
