#[macro_export]
macro_rules! globals_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{Limits, Region};
        use lucet_runtime_internals::instance::InstanceInternal;
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::build::test_module_wasm;

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

            let heap_u32 = unsafe { inst.alloc().heap_u32() };
            assert_eq!(heap_u32[0..=2], [4, 5, 6]);

            inst.run(b"main", &[]).expect("instance runs");

            // now heap should be:
            // [0] = 3
            // [4] = 2
            // [8] = 6

            let heap_u32 = unsafe { inst.alloc().heap_u32() };
            assert_eq!(heap_u32[0..=2], [3, 2, 6]);
        }
    };
}
