#[macro_export]
macro_rules! start_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::{DlModule, Limits, Region};
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::build::test_module_wasm;

        #[test]
        fn global_init() {
            let module =
                test_module_wasm("start", "global_init.wat").expect("module compiled and loaded");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run("main", &[]).expect("instance runs");

            // Now the globals should be:
            // $flossie = 17
            // and heap should be:
            // [0] = 17

            let heap = inst.heap_u32();
            assert_eq!(heap[0], 17);
        }

        #[test]
        fn start_and_call() {
            let module = test_module_wasm("start", "start_and_call.wat")
                .expect("module compiled and loaded");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run("main", &[]).expect("instance runs");

            // Now the globals should be:
            // $flossie = 17
            // and heap should be:
            // [0] = 17

            let heap = inst.heap_u32();
            assert_eq!(heap[0], 17);
        }

        #[test]
        fn no_start() {
            let module =
                test_module_wasm("start", "no_start.wat").expect("module compiled and loaded");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            inst.run("main", &[]).expect("instance runs");

            // Now the globals should be:
            // $flossie = 17
            // and heap should be:
            // [0] = 17

            let heap = inst.heap_u32();
            assert_eq!(heap[0], 17);
        }
    };
}
