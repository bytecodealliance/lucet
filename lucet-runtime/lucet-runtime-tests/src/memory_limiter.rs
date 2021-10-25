#[macro_export]
macro_rules! memory_limiter_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {
        $(
            mod $region_id {
                use lazy_static::lazy_static;
                use lucet_runtime::{DlModule, Limits, Region, RegionCreate, MemoryLimiter};
                use std::sync::Mutex;
                use $TestRegion as TestRegion;
                use $crate::build::test_module_wasm;

                #[test]
                fn ensure_linked() {
                    lucet_runtime::lucet_internal_ensure_linked();
                }

                struct TrivialLimiter;
                #[async_trait::async_trait]
                impl MemoryLimiter for TrivialLimiter {
                    async fn memory_growing(&mut self, _current: usize, _desired: usize) -> bool {
                        true
                    }
                    fn memory_grow_failed(&mut self, _error: &lucet_runtime::Error) {
                    }
                }


                #[test]
                #[should_panic(expected ="instance runs: RuntimeTerminated(TerminationDetails::BlockOnNeedsAsync")]
                fn async_required() {
                    let module = test_module_wasm("memory", "grow_memory.wat")
                        .expect("compile and load grow_memory.wasm");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");


                    inst.set_memory_limiter(Box::new(TrivialLimiter));
                    // This line will panic
                    inst.run("main", &[]).expect("instance runs");
                }

                #[tokio::test]
                async fn trivial_limiter() {
                    let module = test_module_wasm("memory", "grow_memory.wat")
                        .expect("compile and load grow_memory.wasm");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.set_memory_limiter(Box::new(TrivialLimiter));
                    inst.run_async("main", &[], None).await.expect("instance runs");

                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
                    // growing by 1 returns prev size 4
                    assert_eq!(heap[0], 4);
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5);
                }
            }
        )*
    };
}
