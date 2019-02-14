#[macro_export]
macro_rules! globals_tests {
    ( $TestRegion:path ) => {
        /*
        use $TestRegion as TestRegion;
        use $crate::alloc::Limits;
        use $crate::instance::InstanceInternal;
        use $crate::module::ModuleInternal;
        use $crate::region::Region;
        */

        //const INTERNAL_MOD_PATH: &'static str = "lucet-runtime-c/test/build/globals/internal.so";
        //const C_IMPORT_MOD_PATH: &'static str = "lucet-runtime-c/test/build/globals/import.so";
        //const WAT_IMPORT_MOD_PATH: &'static str = "tests/build/globals_guests/import.so";
        //const DEFINITION_SANDBOX_PATH: &'static str = "tests/build/globals_guests/definition.so";

        /* replace with instantiation of module with import global, assert failure
        #[test]
        fn wat_reject_import() {
            let module = DlModule::load_test(WAT_IMPORT_MOD_PATH);
            assert!(module.is_err(), "module load should not succeed");
        }
        */

        /* replace with use of instance public api to make sure defined globals are initialized
         * correctly
        #[test]
        fn read_global0() {
        let module = DlModule::load_test(INTERNAL_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), -1);
        }
        */

    };
}

#[cfg(test)]
mod tests {
    globals_tests!(crate::region::mmap::MmapRegion);
}
