#[macro_export]
macro_rules! memory_tests {
    ( $TestRegion:path ) => {
        use lazy_static::lazy_static;
        use lucet_runtime::{DlModule, Limits, Region};
        use std::sync::Mutex;
        use $TestRegion as TestRegion;
        use $crate::build::test_module_wasm;

        #[test]
        fn current_memory_hostcall() {
            let module = test_module_wasm("memory", "current_memory.wat")
                .expect("compile and load current_memory.wasm");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("main", &[])
                .expect("instance runs")
                .unwrap_returned();
            assert_eq!(u32::from(retval), 4);
        }

        #[test]
        fn grow_memory_hostcall() {
            let module = test_module_wasm("memory", "grow_memory.wat")
                .expect("compile and load grow_memory.wasm");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run("main", &[]).expect("instance runs");

            let heap = inst.heap_u32();
            // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
            // growing by 1 returns prev size 4
            assert_eq!(heap[0], 4);
            // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
            assert_eq!(heap[1], 5);
        }
    };
}
