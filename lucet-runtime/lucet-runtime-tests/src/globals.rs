#[macro_export]
macro_rules! globals_tests {
    ( $TestRegion:path ) => {
        use lucet_module_data::ValueType;
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{Error, Limits, Module, Region};
        use lucet_runtime_internals::module::Signature;
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::build::test_module_wasm;
        use $crate::helpers::{MockExportBuilder, MockModuleBuilder};

        #[test]
        fn defined_globals() {
            let module =
                test_module_wasm("globals", "definition.wat").expect("module compiled and loaded");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            // Now the globals should be:
            // $x = 3
            // $y = 2
            // $z = 6
            // and heap should be:
            // [0] = 4
            // [4] = 5
            // [8] = 6

            let heap_u32 = unsafe { inst.heap_u32() };
            assert_eq!(heap_u32[0..=2], [4, 5, 6]);

            inst.run(b"main", &[]).expect("instance runs");

            // now heap should be:
            // [0] = 3
            // [4] = 2
            // [8] = 6

            let heap_u32 = unsafe { inst.heap_u32() };
            assert_eq!(heap_u32[0..=2], [3, 2, 6]);
        }

        fn mock_import_module() -> Arc<dyn Module> {
            MockModuleBuilder::new()
                .with_import(0, "something", "else")
                .build()
        }

        #[test]
        fn reject_import() {
            let module = mock_import_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            match region.new_instance(module) {
                Ok(_) => panic!("instance creation should not succeed"),
                Err(Error::Unsupported(_)) => (),
                Err(e) => panic!("unexpected error: {}", e),
            }
        }

        fn mock_globals_module() -> Arc<dyn Module> {
            extern "C" {
                fn lucet_vmctx_get_globals(vmctx: *mut lucet_vmctx) -> *mut i64;
            }

            unsafe extern "C" fn get_global0(vmctx: *mut lucet_vmctx) -> i64 {
                let globals = std::slice::from_raw_parts(lucet_vmctx_get_globals(vmctx), 2);
                globals[0]
            }

            unsafe extern "C" fn set_global0(vmctx: *mut lucet_vmctx, val: i64) {
                let globals = std::slice::from_raw_parts_mut(lucet_vmctx_get_globals(vmctx), 2);
                globals[0] = val;
            }

            unsafe extern "C" fn get_global1(vmctx: *mut lucet_vmctx) -> i64 {
                let globals = std::slice::from_raw_parts(lucet_vmctx_get_globals(vmctx), 2);
                globals[1]
            }

            MockModuleBuilder::new()
                .with_global(0, -1)
                .with_global(1, 420)
                .with_export_func(
                    MockExportBuilder::new(b"get_global0", get_global0 as *const extern "C" fn())
                        .with_sig(Signature {
                            params: vec![],
                            ret_ty: Some(ValueType::I64),
                        }),
                )
                .with_export_func(
                    MockExportBuilder::new(b"set_global0", set_global0 as *const extern "C" fn())
                        .with_sig(Signature {
                            params: vec![ValueType::I64],
                            ret_ty: None,
                        }),
                )
                .with_export_func(
                    MockExportBuilder::new(b"get_global1", get_global1 as *const extern "C" fn())
                        .with_sig(Signature {
                            params: vec![],
                            ret_ty: Some(ValueType::I64),
                        }),
                )
                .build()
        }

        /* replace with use of instance public api to make sure defined globals are initialized
         * correctly
         */

        #[test]
        fn globals_initialized() {
            let module = mock_globals_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let inst = region
                .new_instance(module)
                .expect("instance can be created");
            assert_eq!(inst.globals()[0], -1);
            assert_eq!(inst.globals()[1], 420);
        }

        #[test]
        fn get_global0() {
            let module = mock_globals_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), -1);
        }

        #[test]
        fn get_both_globals() {
            let module = mock_globals_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), -1);

            let retval = inst.run(b"get_global1", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), 420);
        }

        #[test]
        fn mutate_global0() {
            let module = mock_globals_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run(b"set_global0", &[666i64.into()])
                .expect("instance runs");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), 666);
        }
    };
}
