use failure::{format_err, Error, ResultExt};
use lucetc::load;
use parity_wasm::elements::Module;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::str;
use tempdir::TempDir;

fn expect_success(o: &Output) -> Result<(), Error> {
    if !o.status.success() {
        let stdout = str::from_utf8(&o.stdout).unwrap();
        let stderr = str::from_utf8(&o.stderr).unwrap();
        Err(format_err!("stdout:{}\nstderr:{}", stdout, stderr))
    } else {
        Ok(())
    }
}

fn cc(input: &PathBuf, output: &PathBuf) -> Result<(), Error> {
    let cc = Command::new(env::var("LUCET_CLANG").unwrap_or("clang".to_owned()))
        .arg("--target=wasm32-wasm")
        .arg("-nostdinc")
        .arg("-fvisibility=default")
        .arg("-c")
        .arg(input)
        .arg("-o")
        .arg(output)
        .output()
        .context(format!("compiling {:?}", input))?;
    expect_success(&cc).context(format!("compiling {:?}", input))?;
    Ok(())
}

fn link(inputs: &[PathBuf], output: &PathBuf) -> Result<(), Error> {
    let mut ld_cmd = Command::new(env::var("LUCET_WASM_LD").unwrap_or("wasm-ld".to_owned()));
    ld_cmd.arg("--no-entry");
    ld_cmd.arg("--allow-undefined");
    ld_cmd.arg("--no-threads");
    for input in inputs {
        ld_cmd.arg(input);
    }
    let ld = ld_cmd
        .arg("-o")
        .arg(output)
        .output()
        .context(format!("linking {:?}", output))?;
    expect_success(&ld).context(format!("linking {:?}", output))?;
    Ok(())
}

fn build_wasm(cfiles: &[PathBuf], libs: &[PathBuf], tempdir: &TempDir) -> Result<PathBuf, Error> {
    let mut objs = Vec::new();
    for c in cfiles {
        let stem = c
            .file_stem()
            .ok_or(format_err!("invalid filename {:?}", c))?
            .to_str()
            .ok_or(format_err!("non-utf8 filename {:?}", c))?;
        let obj = tempdir.path().join(stem).with_extension("o");
        if objs.contains(&obj) {
            return Err(format_err!("non-unique file stem {:?}", c));
        }
        cc(c, &obj)?;
        objs.push(obj);
    }

    let out = tempdir.path().join("out.wasm");
    for lib in libs {
        objs.push(lib.clone());
    }
    link(&objs, &out)?;
    Ok(out)
}

fn test_file_path(name: &str) -> PathBuf {
    PathBuf::from(format!("tests/clang/{}.c", name))
}

fn module_from_c(cfiles: &[&str]) -> Result<Module, Error> {
    let cfiles: Vec<PathBuf> = cfiles.iter().map(|ref f| test_file_path(f)).collect();
    let tempdir = TempDir::new("clang").context("tempdir creation")?;
    let wasm =
        build_wasm(&cfiles, &[], &tempdir).context(format!("building wasm for {:?}", cfiles))?;
    let m = load::read_module(&wasm).context(format!("loading module built from {:?}", cfiles))?;
    Ok(m)
}

use lucetc::bindings::Bindings;
use std::collections::HashMap;

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
        let m = module_from_c(&["empty"]).expect("build module for empty");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for empty");
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 0, "export functions");
        let _c = compile(&p, "empty".into()).expect("compile empty");
    }

    #[test]
    fn just_a() {
        let m = module_from_c(&["a"]).expect("build module for a");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for a");
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 1, "export functions");
        let _c = compile(&p, "a_only".into()).expect("compile a");
    }

    #[test]
    fn just_b() {
        let m = module_from_c(&["b"]).expect("build module for b");
        let b = b_only_test_bindings();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for b");
        assert_eq!(p.import_functions().len(), 1, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 1, "export functions");
        let _c = compile(&p, "b_only".into()).expect("compile b");
    }

    #[test]
    fn a_and_b() {
        let m = module_from_c(&["a", "b"]).expect("build module for a & b");
        let b = Bindings::empty();
        let h = HeapSettings::default();
        let p = Program::new(m, b, h).expect("create program for a & b");
        assert_eq!(p.import_functions().len(), 0, "import functions");
        assert_eq!(num_import_globals(&p), 0, "import globals");
        assert_eq!(num_export_functions(&p), 2, "export functions");
        let _c = compile(&p, "a_and_b".into()).expect("compile a & b");
    }

}
