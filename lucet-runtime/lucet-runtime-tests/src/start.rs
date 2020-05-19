#[macro_export]
macro_rules! start_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {
        $(
            mod $region_id {
                use lucet_runtime::{DlModule, Limits, Region};
                use std::sync::Arc;
                use $TestRegion as TestRegion;
                use $crate::build::test_module_wasm;

                #[test]
                fn ensure_linked() {
                    lucet_runtime::lucet_internal_ensure_linked();
                }

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

                    inst.ensure_signal_handler_installed(false);
                    lucet_runtime::install_lucet_signal_handler();

                    inst.run_start().expect("start section runs");
                    inst.run("main", &[]).expect("instance runs");

                    // Now the globals should be:
                    // $flossie = 17
                    // and heap should be:
                    // [0] = 17

                    let heap = inst.heap_u32();
                    assert_eq!(heap[0], 17);

                    inst.reset().expect("instance resets");

                    let heap = inst.heap_u32();
                    assert_eq!(heap[0], 0);

                    inst.run_start().expect("start section runs again");
                    inst.run("main", &[]).expect("instance runs again");

                    let heap = inst.heap_u32();
                    assert_eq!(heap[0], 17);

                    lucet_runtime::remove_lucet_signal_handler();
                }

                #[test]
                fn manual_sigstack_ok() {
                    test_nonex(|| {
                        use libc::*;
                        use std::mem::MaybeUninit;
                        
                        let mut our_sigstack_alloc = vec![0; lucet_runtime::DEFAULT_SIGNAL_STACK_SIZE];
                        let our_sigstack = stack_t {
                            ss_sp: our_sigstack_alloc.as_mut_ptr() as *mut _,
                            ss_flags: 0,
                            ss_size: lucet_runtime::DEFAULT_SIGNAL_STACK_SIZE,
                        };
                        let mut beforestack = MaybeUninit::<stack_t>::uninit();
                        let beforestack = unsafe {
                            sigaltstack(&our_sigstack, beforestack.as_mut_ptr());
                            beforestack.assume_init()
                        };
                        
                        let module = test_module_wasm("start", "start_and_call.wat")
                            .expect("module compiled and loaded");
                        let limits_no_sigstack = Limits {
                            signal_stack_size: 0,
                            ..Limits::default()
                        };
                        let region =
                            TestRegion::create(1, &limits_no_sigstack).expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");
                        
                        inst.ensure_sigstack_installed(false);
                        
                        inst.run_start().expect("start section runs");
                        inst.run("main", &[]).expect("instance runs");
                        
                        // Now the globals should be:
                        // $flossie = 17
                        // and heap should be:
                        // [0] = 17
                        
                        let heap = inst.heap_u32();
                        assert_eq!(heap[0], 17);
                        
                        inst.reset().expect("instance resets");
                        
                        let heap = inst.heap_u32();
                        assert_eq!(heap[0], 0);
                        
                        inst.run_start().expect("start section runs again");
                        inst.run("main", &[]).expect("instance runs again");
                        
                        let heap = inst.heap_u32();
                        assert_eq!(heap[0], 17);
                        
                        let mut afterstack = MaybeUninit::<stack_t>::uninit();
                        let afterstack = unsafe {
                            sigaltstack(&beforestack, afterstack.as_mut_ptr());
                            afterstack.assume_init()
                        };
                        
                        assert_eq!(afterstack.ss_sp, our_sigstack_alloc.as_mut_ptr() as *mut _);
                    });
                }
            }
        )*
    };
}
