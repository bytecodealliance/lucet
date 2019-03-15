use failure::{Error, ResultExt};
use lucet_wasi_sdk::Link;
use lucetc::bindings::Bindings;
use lucetc::load;
use parity_wasm::elements::Module;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str;
use tempfile;

fn module_from_c(cfiles: &[&str], exports: &[&str]) -> Result<Module, Error> {
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
        .cflag("-nostartfiles")
        .ldflag("--no-entry")
        .ldflag("--allow-undefined");
    for export in exports {
        linker.with_ldflag(&format!("--export={}", export));
    }
    linker.link(wasm.clone())?;

    let m = load::read_module(&wasm).context(format!("loading module built from {:?}", cfiles))?;
    Ok(m)
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
    use cranelift_module::Linkage;
    use lucetc::bindings::Bindings;
    use lucetc::compile;
    use lucetc::compiler::OptLevel;
    use lucetc::program::{HeapSettings, Program};

    fn num_import_globals(p: &Program) -> usize {
        p.globals()
            .iter()
            .filter_map(|g| g.as_import())
            .collect::<Vec<_>>()
            .len()
    }

    fn num_export_functions(p: &Program) -> usize {
        p.defined_functions()
            .iter()
            .filter(|f| f.linkage() == Linkage::Export)
            .collect::<Vec<_>>()
            .len()
    }

    #[test]
    fn empty() {
        let m = module_from_c(&["empty"], &[]).expect("build module for empty");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for empty");
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 0, "export functions");
        let _c = compile(&p, "empty".into(), OptLevel::Best).expect("compile empty");
    }

    #[test]
    fn just_a() {
        let m = module_from_c(&["a"], &["a"]).expect("build module for a");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for a");
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 1, "export functions");
        let _c = compile(&p, "a_only".into(), OptLevel::Best).expect("compile a");
    }

    #[test]
    fn just_b() {
        let m = module_from_c(&["b"], &["b"]).expect("build module for b");
        let b = b_only_test_bindings();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for b");
        assert_eq!(p.import_functions().len(), 1, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 1, "export functions");
        let _c = compile(&p, "b_only".into(), OptLevel::Best).expect("compile b");
    }

    #[test]
    fn a_and_b() {
        let m = module_from_c(&["a", "b"], &["a", "b"]).expect("build module for a & b");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for a & b");
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 2, "export functions");
        let _c = compile(&p, "a_and_b".into(), OptLevel::Best).expect("compile a & b");
    }

}
