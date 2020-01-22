use anyhow::Error;
use lucet_runtime_internals::module::DlModule;
use lucetc::Lucetc;
use std::sync::Arc;
use tempfile::TempDir;

pub fn stack_testcase(num_locals: usize) -> Result<Arc<DlModule>, Error> {
    let native_build = Lucetc::try_from_bytes(generate_test_wat(num_locals))?;

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}

fn generate_test_wat(num_locals: usize) -> String {
    assert!(num_locals > 2);

    let mut module =
        "(module (func $localpalooza (export \"localpalooza\") (param i32) (result i32)\n"
            .to_owned();

    // Declare locals:
    module.push_str("(local ");
    for _ in 0..num_locals {
        module.push_str("i32 ");
    }
    module.push_str(")\n");

    // Use each local for the first time:
    for i in 1..num_locals {
        module.push_str(&format!(
            "(set_local {} (i32.add (get_local {}) (i32.xor (get_local {}) (i32.const {}))))\n",
            i,
            i - 1,
            i,
            i
        ));
    }

    // Use each local for a second time, so they get pushed to the stack between uses:
    for i in 2..(num_locals - 1) {
        module.push_str(&format!(
            "(set_local {} (i32.add (get_local {}) (i32.and (get_local {}) (i32.const {}))))\n",
            i,
            i - 1,
            i,
            i
        ));
    }

    // Keep locals alive across a recursive call. Make as many recursive calls as the first
    // argument to the func:
    module.push_str("(if (get_local 0)\n");
    module.push_str(&format!(
        "  (then (set_local {} (i32.add (get_local {})\n",
        num_locals - 1,
        num_locals - 2
    ));
    module.push_str("      (call $localpalooza (i32.sub (get_local 0) (i32.const 1))))))\n");
    module.push_str(&format!(
        "  (else (set_local {} (i32.add (get_local {}) (get_local {})))))\n",
        num_locals - 1,
        num_locals - 2,
        num_locals - 3
    ));

    module.push_str(&format!("(get_local {})\n", num_locals - 1));
    module.push_str("))\n");
    module
}

#[macro_export]
macro_rules! stack_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::{
            DlModule, Error, InstanceHandle, Limits, Region, TrapCode, UntypedRetVal, Val,
        };
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::stack::stack_testcase;

        fn run(module: Arc<DlModule>, recursion_depth: i32) -> Result<UntypedRetVal, Error> {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run("localpalooza", &[recursion_depth.into()])
                .and_then(|rr| rr.returned())
        }

        fn expect_ok(module: Arc<DlModule>, recursion_depth: i32) {
            assert!(run(module, recursion_depth).is_ok());
        }

        fn expect_stack_overflow(module: Arc<DlModule>, recursion_depth: i32, probestack: bool) {
            match run(module, recursion_depth) {
                Err(Error::RuntimeFault(details)) => {
                    // We should get a nonfatal trap due to the stack overflow.
                    assert_eq!(details.fatal, false);
                    assert_eq!(details.trapcode, Some(TrapCode::StackOverflow));
                    if probestack {
                        // Make sure we overflowed in the stack probe as expected
                        //
                        // TODO: this no longer differentiates between different stack overflow
                        // sites after moving the stack probe into lucetc; figure out a way to
                        // provide that information or just wait till we can do the stack probe as a
                        // global symbol again
                        let addr_details =
                            details.rip_addr_details.expect("can look up addr details");
                        assert!(addr_details.in_module_code);
                    }
                }
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn expect_ok_locals3_1() {
            expect_ok(stack_testcase(3).expect("generate stack_testcase 3"), 1);
        }

        // The test with 64 locals should cause a stack overflow on the 481st recursion.  The trap
        // table knows about all of the instructions in the function that manipulate the stack, so
        // the catch mechanism for this is the usual one.

        #[test]
        fn expect_ok_locals64_1() {
            expect_ok(stack_testcase(64).expect("generate stack_testcase 64"), 1);
        }

        #[test]
        fn expect_ok_locals64_2() {
            expect_ok(stack_testcase(64).expect("generate stack_testcase 64"), 2);
        }

        #[test]
        fn expect_ok_locals64_481() {
            // We use 64 local variables, but cranelift optimizes many of them into registers
            // rather than stack space. Four of those are callee-saved, for n > 18 local
            // variables, we actually use n + 4 stack spaces. So we use 64 spaces at 64 - 4 = 60
            // local variables.
            expect_ok(
                stack_testcase(64 - 4).expect("generate stack_testcase 64"),
                480,
            );
        }

        #[test]
        fn expect_stack_overflow_locals64_481() {
            expect_stack_overflow(
                // Same note as `expect_ok_locals64_481`
                stack_testcase(64 - 4).expect("generate stack_testcase 64"),
                481,
                true,
            );
        }

        // 1050 locals is about 1 page (4k) on the stack - just enough for Cranelift to use probestack to grow
        // the stack. The 31st recursion should cause a stack overflow.

        #[test]
        fn expect_ok_locals_1page_1() {
            expect_ok(
                stack_testcase(1050).expect("generate stack_testcase 1050"),
                1,
            );
        }

        #[test]
        fn expect_ok_locals_1page_2() {
            expect_ok(
                stack_testcase(1050).expect("generate stack_testcase 1050"),
                2,
            );
        }

        #[test]
        fn expect_ok_locals_1page_30() {
            expect_ok(
                stack_testcase(1050).expect("generate stack_testcase 1050"),
                30,
            );
        }

        #[test]
        fn expect_stack_overflow_locals_1page_31() {
            expect_stack_overflow(
                stack_testcase(1050).expect("generate stack_testcase 1050"),
                31,
                true,
            );
        }

        // This test has 5000 locals - over 4 pages worth. Cranelift will use probestack here as well. The
        // 6th recursion should cause a stack overflow.

        #[test]
        fn expect_ok_locals_multipage_1() {
            expect_ok(
                stack_testcase(5000).expect("generate stack_testcase 5000"),
                1,
            );
        }

        #[test]
        fn expect_ok_locals_multipage_2() {
            expect_ok(
                stack_testcase(5000).expect("generate stack_testcase 5000"),
                2,
            );
        }

        #[test]
        fn expect_ok_locals_multipage_5() {
            expect_ok(
                stack_testcase(5000).expect("generate stack_testcase 5000"),
                5,
            );
        }

        #[test]
        fn expect_stack_overflow_locals_multipage_6() {
            expect_stack_overflow(
                stack_testcase(5000).expect("generate stack_testcase 5000"),
                6,
                true,
            );
        }
    };
}
