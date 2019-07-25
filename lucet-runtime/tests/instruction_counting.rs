use lucet_runtime::{Error, Limits, DlModule, MmapRegion, Region};
use lucetc::{Lucetc, LucetcOpts};
use std::path::Path;
use std::sync::Arc;
use std::fs::DirEntry;
use tempfile::TempDir;
use rayon::prelude::*;

pub fn wasm_test<P: AsRef<Path>>(wasm_file: P) -> Result<Arc<DlModule>, Error> {
    let workdir = TempDir::new().expect("create working directory");

    let native_build = Lucetc::new(wasm_file).with_count_instructions(true);

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}

#[test]
pub fn check_instruction_counts() {
    let files: Vec<DirEntry> = std::fs::read_dir("./tests/instruction_counting")
        .expect("can iterate test files")
        .map(|ent| {
            let ent = ent.expect("can get test files");
            assert!(ent.file_type().unwrap().is_file(), "directories not supported in test/instruction_counting");
            ent
        })
        .collect();

    assert!(
        files.len() > 0,
        "there are no test cases in the `instruction_counting` directory"
    );

    files.par_iter().for_each(|ent| {
        let wasm_path = ent.path();
        let module = wasm_test(&wasm_path).expect("can load module");

        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

        let mut inst = region
            .new_instance(module)
            .expect("instance can be created");

        inst.run("test_function", &[]).expect("instance runs");

        let instruction_count = inst.get_instruction_count();

        assert_eq!(
            instruction_count,
            inst.run("instruction_count", &[])
                .expect("instance still runs")
                .as_i64() as u64,
            "instruction count for test case {} is incorrect",
            wasm_path.display()
        );
    });
}
