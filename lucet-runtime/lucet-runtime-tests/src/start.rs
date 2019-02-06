#[macro_export]
macro_rules! start_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::{DlModule, Limits, Region};
        use $TestRegion as TestRegion;
        use $crate::helpers::DlModuleExt;

        const GLOBAL_INIT_SANDBOX_PATH: &'static str = "tests/build/start_guests/global_init.so";
        const START_AND_CALL_SANDBOX_PATH: &'static str =
            "tests/build/start_guests/start_and_call.so";
        const NO_START_SANDBOX_PATH: &'static str = "tests/build/start_guests/no_start.so";

        #[test]
        fn global_init() {
            let module = DlModule::load_test(GLOBAL_INIT_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            // Now the globals should be:
            // $flossie = 17
            // and heap should be:
            // [0] = 17

            let heap = inst.heap_u32();
            assert_eq!(heap[0], 17);
        }

        #[test]
        fn start_and_call() {
            let module = DlModule::load_test(START_AND_CALL_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            // Now the globals should be:
            // $flossie = 17
            // and heap should be:
            // [0] = 17

            let heap = inst.heap_u32();
            assert_eq!(heap[0], 17);
        }

        #[test]
        fn no_start() {
            let module = DlModule::load_test(NO_START_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            // Now the globals should be:
            // $flossie = 17
            // and heap should be:
            // [0] = 17

            let heap = inst.heap_u32();
            assert_eq!(heap[0], 17);
        }
    };
}
