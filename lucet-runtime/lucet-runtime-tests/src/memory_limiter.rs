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


                // If you try to use MemoryLimiter without Instance::run_async, it will panic.
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

                // Use a trivial memory limiter, one which instantly returns `true`
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

                // Use a slightly less trivial limiter, one which will `.await` for a millisecond
                // and then return `true`
                #[tokio::test]
                async fn await_inside_limiter() {

                    struct AwaitInsideLimiter;
                    #[async_trait::async_trait]
                    impl MemoryLimiter for AwaitInsideLimiter {
                        async fn memory_growing(&mut self, _current: usize, _desired: usize) -> bool {
                            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                            true
                        }
                        fn memory_grow_failed(&mut self, _error: &lucet_runtime::Error) {
                        }
                    }

                    let module = test_module_wasm("memory", "grow_memory.wat")
                        .expect("compile and load grow_memory.wasm");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.set_memory_limiter(Box::new(AwaitInsideLimiter));
                    inst.run_async("main", &[], None).await.expect("instance runs");

                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
                    // growing by 1 returns prev size 4
                    assert_eq!(heap[0], 4);
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5);
                }

                // Use a limiter that panics, show that this panic propogates (doesnt crash and
                // make things worse)
                #[tokio::test]
                #[should_panic(expected = "panic! at the memory_growing")]
                async fn panic_inside_limiter() {

                    struct PanicInsideLimiter;
                    #[async_trait::async_trait]
                    impl MemoryLimiter for PanicInsideLimiter {
                        async fn memory_growing(&mut self, _current: usize, _desired: usize) -> bool {
                            panic!("panic! at the memory_growing");
                        }
                        fn memory_grow_failed(&mut self, _error: &lucet_runtime::Error) {
                        }
                    }

                    let module = test_module_wasm("memory", "grow_memory.wat")
                        .expect("compile and load grow_memory.wasm");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.set_memory_limiter(Box::new(PanicInsideLimiter));
                    inst.run_async("main", &[], None).await.expect("instance runs");

                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
                    // growing by 1 returns prev size 4
                    assert_eq!(heap[0], 4);
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5);
                }

                // Use a useful limiter, one which lets you grow up to 5 pages but no
                // further. It also records the error for inspection.
                #[tokio::test]
                async fn useful_limiter() {
                    struct Inner {
                        max_bytes: usize,
                        error: std::sync::Mutex<Option<String>>,
                    };
                    struct UsefulLimiter(std::sync::Arc<Inner>);
                    #[async_trait::async_trait]
                    impl MemoryLimiter for UsefulLimiter {
                        async fn memory_growing(&mut self, _current: usize, desired: usize) -> bool {
                            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                            desired <= self.0.max_bytes
                        }
                        fn memory_grow_failed(&mut self, error: &lucet_runtime::Error) {
                            *self.0.error.lock().unwrap() = Some(format!("{:?}", error));
                        }
                    }

                    let module = test_module_wasm("memory", "grow_memory.wat")
                        .expect("compile and load grow_memory.wasm");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    let limiter = std::sync::Arc::new(Inner {
                        // Maximum: 5 pages
                        max_bytes: 5 * 64 * 1024,
                        error: std::sync::Mutex::new(None),
                    });

                    inst.set_memory_limiter(Box::new(UsefulLimiter(std::sync::Arc::clone(&limiter))));
                    inst.run_async("main", &[], None).await.expect("instance runs");

                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
                    // growing by 1 returns prev size 4
                    assert_eq!(heap[0], 4);
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5);
                    drop(heap);

                    assert_eq!(*limiter.error.lock().unwrap(), None, "no limiter error on first run");

                    inst.run_async("main", &[], None).await.expect("instance runs a second time");
                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; this should
                    // fail and be -1
                    assert_eq!(heap[0], -1i32 as u32);
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5);

                    assert_eq!(*limiter.error.lock().unwrap(), Some("InternalError(memory limiter denied growth)".to_owned()), "limiter error on second run");

                }

                #[tokio::test]
                async fn record_limits_failure() {
                    struct Inner {
                        error: std::sync::Mutex<Option<String>>,
                    };
                    struct UsefulLimiter(std::sync::Arc<Inner>);
                    #[async_trait::async_trait]
                    impl MemoryLimiter for UsefulLimiter {
                        async fn memory_growing(&mut self, _current: usize, desired: usize) -> bool {
                            true
                        }
                        fn memory_grow_failed(&mut self, error: &lucet_runtime::Error) {
                            *self.0.error.lock().unwrap() = Some(format!("{:?}", error));
                        }
                    }

                    // Limit to 5 pages with Limits, rather than with MemoryLimiter
                    let limits = Limits {
                        heap_memory_size: 5 * 64 * 1024,
                        ..Limits::default()
                    };

                    let module = test_module_wasm("memory", "grow_memory.wat")
                        .expect("compile and load grow_memory.wasm");
                    let region = <TestRegion as RegionCreate>::create(1, &limits).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    let limiter = std::sync::Arc::new(Inner {
                        error: std::sync::Mutex::new(None)
                    });

                    inst.set_memory_limiter(Box::new(UsefulLimiter(std::sync::Arc::clone(&limiter))));
                    inst.run_async("main", &[], None).await.expect("instance runs");

                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
                    // growing by 1 returns prev size 4
                    assert_eq!(heap[0], 4, "grow memory on first run");
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5, "current memory on first run");
                    drop(heap);

                    assert_eq!(*limiter.error.lock().unwrap(), None, "no limiter error on first run");

                    inst.run_async("main", &[], None).await.expect("instance runs a second time");
                    let heap = inst.heap_u32();
                    // guest puts the result of the grow_memory(1) call in heap[0]; this should
                    // fail and be -1
                    assert_eq!(heap[0], -1i32 as u32, "grow memory on second run");
                    // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
                    assert_eq!(heap[1], 5, "current memory on second run");

                    let error = limiter.error.lock().unwrap().take();
                    assert_eq!(error, Some(format!("LimitsExceeded(\"expansion would exceed runtime-specified heap limit: {:?}\")", limits)))
                }

            }
        )*
    };
}
