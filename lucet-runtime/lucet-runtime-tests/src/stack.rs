#[macro_export]
macro_rules! stack_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::{
            DlModule, Error, InstanceHandle, Limits, Region, TrapCodeType, UntypedRetVal, Val,
        };
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::helpers::DlModuleExt;

        const LOCALS64_SANDBOX_PATH: &'static str = "tests/build/stack_guests/locals_64.so";
        const LOCALS_1PAGE_SANDBOX_PATH: &'static str = "tests/build/stack_guests/locals_1page.so";
        const LOCALS_MULTIPAGE_SANDBOX_PATH: &'static str =
            "tests/build/stack_guests/locals_multipage.so";

        fn run(path: &str, recursion_depth: libc::c_int) -> Result<UntypedRetVal, Error> {
            let module = DlModule::load_test(path).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run(b"localpalooza", &[recursion_depth.into()])
        }

        fn expect_ok(path: &str, recursion_depth: libc::c_int) {
            assert!(run(path, recursion_depth).is_ok());
        }

        fn expect_stack_overflow(path: &str, recursion_depth: libc::c_int, probestack: bool) {
            match run(path, recursion_depth) {
                Err(Error::RuntimeFault(details)) => {
                    // We should get a nonfatal trap due to the stack overflow.
                    assert_eq!(details.fatal, false);
                    assert_eq!(details.trapcode.ty, TrapCodeType::StackOverflow);
                    if probestack {
                        // When the runtime catches probestack, it puts this special tag in the trapcode
                        assert_eq!(details.trapcode.tag, std::u16::MAX);
                    }
                }
                res => panic!("unexpected result: {:?}", res),
            }
        }

        // The test with 64 locals should take up 252 bytes per stack frame. Along with the overhead for the
        // sandbox, that means it should overflow on the 455th recursion.  The trap table knows about all of
        // the instructions in the function that manipulate the stack, so the catch mechanism for this is
        // the usual one.

        #[test]
        fn expect_ok_locals64_1() {
            expect_ok(LOCALS64_SANDBOX_PATH, 1);
        }

        #[test]
        fn expect_ok_locals64_2() {
            expect_ok(LOCALS64_SANDBOX_PATH, 2);
        }

        #[test]
        fn expect_ok_locals64_454() {
            expect_ok(LOCALS64_SANDBOX_PATH, 454);
        }

        #[test]
        fn expect_stack_overflow_locals64_455() {
            expect_stack_overflow(LOCALS64_SANDBOX_PATH, 455, false);
        }

        // This test has about 1 page worth of locals - just enough for Cranelift to use probestack to grow
        // the stack. The 31st recursion should cause a stack overflow.

        #[test]
        fn expect_ok_locals_1page_1() {
            expect_ok(LOCALS_1PAGE_SANDBOX_PATH, 1);
        }

        #[test]
        fn expect_ok_locals_1page_2() {
            expect_ok(LOCALS_1PAGE_SANDBOX_PATH, 2);
        }

        #[test]
        fn expect_ok_locals_1page_30() {
            expect_ok(LOCALS_1PAGE_SANDBOX_PATH, 30);
        }

        #[test]
        fn expect_stack_overflow_locals_1page_31() {
            expect_stack_overflow(LOCALS_1PAGE_SANDBOX_PATH, 31, true);
        }

        // This test has 5000 locals - over 4 pages worth. Cranelift will use probestack here as well. The
        // 6th recursion should cause a stack overflow.

        #[test]
        fn expect_ok_locals_multipage_1() {
            expect_ok(LOCALS_MULTIPAGE_SANDBOX_PATH, 1);
        }

        #[test]
        fn expect_ok_locals_multipage_2() {
            expect_ok(LOCALS_MULTIPAGE_SANDBOX_PATH, 2);
        }

        #[test]
        fn expect_ok_locals_multipage_5() {
            expect_ok(LOCALS_MULTIPAGE_SANDBOX_PATH, 5);
        }

        #[test]
        fn expect_stack_overflow_locals_multipage_6() {
            expect_stack_overflow(LOCALS_MULTIPAGE_SANDBOX_PATH, 6, true);
        }
    };
}
