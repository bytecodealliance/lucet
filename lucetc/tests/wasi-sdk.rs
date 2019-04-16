use failure::{Error, ResultExt};
use lucet_wasi_sdk::Link;
use lucetc::Bindings;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str;
use tempfile;

fn module_from_c(cfiles: &[&str], exports: &[&str]) -> Result<Vec<u8>, Error> {
    let cfiles: Vec<PathBuf> = cfiles
        .iter()
        .map(|ref name| PathBuf::from(format!("tests/wasi-sdk/{}.c", name)))
        .collect();
    let tempdir = tempfile::Builder::new()
        .prefix("wasi-sdk-test")
        .tempdir()
        .context("tempdir creation")?;

    let mut wasm = PathBuf::from(tempdir.path());
    wasm.push("out.wasm");

    let mut linker = Link::new(&cfiles)
        .with_cflag("-nostartfiles")
        .with_ldflag("--no-entry")
        .with_ldflag("--allow-undefined");
    for export in exports {
        linker.ldflag(&format!("--export={}", export));
    }
    linker.link(wasm.clone())?;

    let mut wasm_file = File::open(wasm)?;
    let mut wasm_contents = Vec::new();
    wasm_file.read_to_end(&mut wasm_contents)?;
    Ok(wasm_contents)
}

fn b_only_test_bindings() -> Bindings {
    let imports: HashMap<String, String> = [
        ("a".into(), "a".into()), // b_only
    ]
    .iter()
    .cloned()
    .collect();

    Bindings::env(imports)
}

mod programs {
    use super::{b_only_test_bindings, module_from_c};
    use lucetc::{Bindings, Compiler, HeapSettings, OptLevel};

    #[test]
    fn empty() {
        let m = module_from_c(&["empty"], &[]).expect("build module for empty");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Best, &b, h).expect("compile empty");
        let mdata = c.module_data();
        assert!(mdata.heap_spec().is_some());
        // clang creates 3 globals, all internal:
        assert_eq!(mdata.globals_spec().len(), 3);
        assert_eq!(
            mdata
                .globals_spec()
                .iter()
                .filter(|g| g.export().is_some())
                .collect::<Vec<_>>()
                .len(),
            0
        );

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 0, "export functions");
        */

        let _obj = c.object_file().expect("generate code from empty");
    }

    #[test]
    fn just_a() {
        let m = module_from_c(&["a"], &["a"]).expect("build module for a");

        let b = Bindings::empty();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Best, &b, h).expect("compile a");
        let _mdata = c.module_data();

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 1, "export functions");
        */

        let _obj = c.object_file().expect("generate code from a");
    }

    #[test]
    fn just_b() {
        let m = module_from_c(&["b"], &["b"]).expect("build module for b");
        let b = b_only_test_bindings();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Best, &b, h).expect("compile b");
        let _mdata = c.module_data();
        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(p.import_functions().len(), 1, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 1, "export functions");
        */
        let _obj = c.object_file().expect("generate code from b");
    }

    #[test]
    fn a_and_b() {
        let m = module_from_c(&["a", "b"], &["a", "b"]).expect("build module for a & b");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Best, &b, h).expect("compile a & b");
        let _mdata = c.module_data();
        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 2, "export functions");
        */
        let _obj = c.object_file().expect("generate code from a & b");
    }

}
