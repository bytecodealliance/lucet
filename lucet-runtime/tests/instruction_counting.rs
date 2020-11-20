use anyhow::Error;
use lucet_runtime::{DlModule, Limits, MmapRegion, Region, RunResult};
use lucetc::{Lucetc, LucetcOpts};
use rayon::prelude::*;
use std::fs::DirEntry;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tempfile::TempDir;

pub fn wasm_test<P: AsRef<Path>>(
    wasm_file: P,
    icount_option: bool,
) -> Result<Arc<DlModule>, Error> {
    let workdir = TempDir::new().expect("create working directory");

    let native_build = Lucetc::new(wasm_file).with_count_instructions(icount_option);

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}

pub fn get_instruction_count_test_files() -> Vec<DirEntry> {
    std::fs::read_dir("./tests/instruction_counting")
        .expect("can iterate test files")
        .map(|ent| {
            let ent = ent.expect("can get test files");
            assert!(
                ent.file_type().unwrap().is_file(),
                "directories not supported in test/instruction_counting"
            );
            ent
        })
        .collect()
}

#[test]
pub fn check_instruction_count_off() {
    let files: Vec<DirEntry> = get_instruction_count_test_files();

    assert!(
        !files.is_empty(),
        "there are no test cases in the `instruction_counting` directory"
    );

    files.par_iter().for_each(|ent| {
        let wasm_path = ent.path();
        let do_not_instrument = false;
        let module = wasm_test(&wasm_path, do_not_instrument).expect("can load module");

        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

        let mut inst = region
            .new_instance(module)
            .expect("instance can be created");

        inst.run("test_function", &[]).expect("instance runs");

        let instruction_count = inst.get_instruction_count();
        if instruction_count.is_some() {
            panic!("instruction count instrumentation was not expected from instance");
        }
    });
}

#[test]
pub fn check_instruction_count() {
    let files: Vec<DirEntry> = get_instruction_count_test_files();

    assert!(
        !files.is_empty(),
        "there are no test cases in the `instruction_counting` directory"
    );

    files.par_iter().for_each(|ent| {
        let wasm_path = ent.path();
        let do_instrument = true;
        let module = wasm_test(&wasm_path, do_instrument).expect("can load instrumented module");

        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

        let mut inst = region
            .new_instance(module)
            .expect("instance can be created");

        inst.run("test_function", &[]).expect("instance runs");

        let instruction_count = inst
            .get_instruction_count()
            .expect("instruction count expected from instance");

        assert_eq!(
            instruction_count,
            match inst
                .run("instruction_count", &[])
                .expect("instance still runs")
            {
                RunResult::Returned(value) => value.as_i64() as u64,
                RunResult::Yielded(_) => {
                    panic!("instruction counting test runner doesn't support yielding");
                }
            },
            "instruction count for test case {} is incorrect",
            wasm_path.display()
        );
    });
}

/// Create a fake `Waker` for testing a `Future`.
///
/// Borrowed from:
/// https://stackoverflow.com/questions/63263880/testing-futures-and-streams-how-do-i-create-a-fake-context
fn dummy_waker() -> Waker {
    use std::task::{RawWaker, RawWakerVTable};

    static DUMMY_VTABLE: RawWakerVTable =
        RawWakerVTable::new(dummy_clone, dummy_wake, dummy_wake_by_ref, dummy_drop);

    unsafe fn dummy_clone(ptr: *const ()) -> RawWaker {
        RawWaker::new(ptr, &DUMMY_VTABLE)
    }
    unsafe fn dummy_wake(_ptr: *const ()) {}

    unsafe fn dummy_wake_by_ref(_ptr: *const ()) {}

    unsafe fn dummy_drop(_ptr: *const ()) {}

    unsafe { Waker::from_raw(RawWaker::new(&(), &DUMMY_VTABLE)) }
}

#[test]
pub fn check_instruction_count_with_periodic_yields() {
    let files: Vec<DirEntry> = get_instruction_count_test_files();

    assert!(
        !files.is_empty(),
        "there are no test cases in the `instruction_counting` directory"
    );

    files.par_iter().for_each(|ent| {
        let wasm_path = ent.path();
        let do_instrument = true;
        let module = wasm_test(&wasm_path, do_instrument).expect("can load instrumented module");

        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

        let mut inst = region
            .new_instance(module)
            .expect("instance can be created");

        let yields = {
            let mut yields = 0;
            let mut future = Box::pin(inst.run_async("test_function", &[], Some(1000)));
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            loop {
                match future.as_mut().poll(&mut context) {
                    Poll::Ready(val) => {
                        val.expect("instance runs");
                        break;
                    }
                    Poll::Pending => {
                        yields += 1;
                        if yields > 1000 {
                            panic!("Instruction-counting test ran for too long");
                        }
                    }
                }
            }
            yields
        };

        let instruction_count = inst
            .get_instruction_count()
            .expect("instruction count expected from instance");

        assert_eq!(
            instruction_count,
            match inst
                .run("instruction_count", &[])
                .expect("instance still runs")
            {
                RunResult::Returned(value) => value.as_i64() as u64,
                RunResult::Yielded(_) => {
                    panic!("instruction counting test runner doesn't support yielding");
                }
            },
            "instruction count for test case {} is incorrect",
            wasm_path.display()
        );

        // There is some variance to this in the real world, because yields happen only at the ends
        // of basic blocks, and there may be an off-by-one; so allow a small range.
        let expected_yields = instruction_count / 1000;
        let diff = if expected_yields > yields {
            expected_yields - yields
        } else {
            yields - expected_yields
        };
        assert!(
            diff <= 1,
            "periodic yield count for test case {} is incorrect",
            wasm_path.display()
        );
    });
}

#[test]
fn ensure_linked() {
    lucet_runtime::lucet_internal_ensure_linked();
}
