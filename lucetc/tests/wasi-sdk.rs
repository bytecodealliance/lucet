use failure::{Error, ResultExt};
use lucet_wasi_sdk::{CompileOpts, Link, LinkOpt, LinkOpts};
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
        let c = Compiler::new(&m, OptLevel::Fast, &b, h).expect("compile empty");
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

    #[test]
    fn just_a() {
        let m = module_from_c(&["a"], &["a"]).expect("build module for a");

        let b = Bindings::empty();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Fast, &b, h).expect("compile a");
        let mdata = c.module_data().unwrap();

        assert_eq!(mdata.import_functions().len(), 0, "import functions");
        assert_eq!(mdata.export_functions().len(), 1, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */

        let _obj = c.object_file().expect("generate code from a");
    }

    #[test]
    fn just_b() {
        let m = module_from_c(&["b"], &["b"]).expect("build module for b");
        let b = b_only_test_bindings();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Fast, &b, h).expect("compile b");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.import_functions().len(), 1, "import functions");
        assert_eq!(mdata.export_functions().len(), 1, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */
        let _obj = c.object_file().expect("generate code from b");
    }

    #[test]
    fn a_and_b() {
        let m = module_from_c(&["a", "b"], &["a", "b"]).expect("build module for a & b");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let c = Compiler::new(&m, OptLevel::Fast, &b, h).expect("compile a & b");
        let mdata = c.module_data().unwrap();
        assert_eq!(mdata.import_functions().len(), 0, "import functions");
        assert_eq!(mdata.export_functions().len(), 2, "export functions");

        /* FIXME: module data doesn't contain the information to check these properties:
        assert_eq!(num_import_globals(&p), 0, "import globals");
        */
        let _obj = c.object_file().expect("generate code from a & b");
    }

}
