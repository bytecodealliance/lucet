#[macro_export]
macro_rules! globals_tests {
    ( $TestRegion:path ) => {
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::alloc::Limits;
        use $crate::error::Error;
        use $crate::instance::InstanceInternal;
        use $crate::module::{DlModule, MockModuleBuilder, Module};
        use $crate::region::Region;
        use $crate::vmctx::{lucet_vmctx, Vmctx};

        const DEFINITION_SANDBOX_PATH: &'static str = "tests/build/globals_guests/definition.so";

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
            extern "C" fn get_global0(vmctx: *mut lucet_vmctx) -> i64 {
                unsafe { Vmctx::from_raw(vmctx) }.globals()[0]
            }

            extern "C" fn set_global0(vmctx: *mut lucet_vmctx, val: i64) {
                unsafe { Vmctx::from_raw(vmctx) }.globals_mut()[0] = val;
            }

            extern "C" fn get_global1(vmctx: *mut lucet_vmctx) -> i64 {
                unsafe { Vmctx::from_raw(vmctx) }.globals()[1]
            }

            MockModuleBuilder::new()
                .with_global(0, -1)
                .with_global(1, 420)
                .with_export_func(b"get_global0", get_global0 as *const extern "C" fn())
                .with_export_func(b"set_global0", set_global0 as *const extern "C" fn())
                .with_export_func(b"get_global1", get_global1 as *const extern "C" fn())
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

#[cfg(test)]
mod tests {
    globals_tests!(crate::region::mmap::MmapRegion);
}
