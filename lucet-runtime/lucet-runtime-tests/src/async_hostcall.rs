

use std::future::Future;
use std::task::{Waker, Context, Poll};
use std::sync::{Arc, Mutex};

enum StubFutureInner {
    NeverPolled,
    Polled(Waker),
    Ready
}
#[derive(Clone)]
pub struct StubFuture(Arc<Mutex<StubFutureInner>>);
impl StubFuture {
    pub fn new() -> Self { StubFuture(Arc::new(Mutex::new(StubFutureInner::NeverPolled))) }
    pub fn make_ready(&self) {
        let mut inner = self.0.lock().unwrap();
        match std::mem::replace(&mut *inner, StubFutureInner::Ready) {
            StubFutureInner::Polled(waker) => {
                waker.wake();
            }
            _ => panic!("never polled")
        }
    }
}

impl Future for StubFuture {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.0.lock().unwrap();

        match *inner {
            StubFutureInner::Ready => Poll::Ready(()),
            _ => {
                *inner = StubFutureInner::Polled(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}


#[macro_export]
macro_rules! async_hostcall_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {
        use lucet_runtime::{vmctx::Vmctx, lucet_hostcall};
        use std::future::Future;
        use std::task::{Waker, Context};
        use $crate::async_hostcall::StubFuture;


        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_containing_block_on(vmctx: &Vmctx, value: u32) {
            let asynced_value = vmctx.block_on(async move { value });
            assert_eq!(asynced_value, value);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_containing_yielding_block_on(vmctx: &Vmctx, times: u32) {
            struct YieldingFuture { times: u32 }

            impl Future for YieldingFuture {
                type Output = ();

                fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                    if self.times == 0 {
                        return std::task::Poll::Ready(())
                    } else {
                        self.get_mut().times -= 1;

                        cx.waker().wake_by_ref();

                        return std::task::Poll::Pending
                    }
                }
            }

            vmctx.block_on(YieldingFuture { times });
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub async fn hostcall_async_containing_yielding_block_on(vmctx: &Vmctx, times: u32, times_double: u32) -> u32 {
            assert_eq!(times * 2, times_double);

            struct YieldingFuture { times: u32 }

            impl Future for YieldingFuture {
                type Output = ();

                fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                    if self.times == 0 {
                        return std::task::Poll::Ready(())
                    } else {
                        self.get_mut().times -= 1;

                        cx.waker().wake_by_ref();

                        return std::task::Poll::Pending
                    }
                }
            }

            for i in 0..times {
                YieldingFuture { times: 2 }.await
            }

            return times * 2;
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub async fn await_manual_future(vmctx: &Vmctx) {
            vmctx.yield_();
            vmctx.get_embed_ctx_mut::<Option<StubFuture>>().take().unwrap().await;
        }

        $(
            mod $region_id {
                use lucet_runtime::{DlModule, Error, Limits, Region, RegionCreate, TerminationDetails, RunResult};
                use std::sync::{Arc};
                use $TestRegion as TestRegion;
                use $crate::build::test_module_c;

                use $crate::async_hostcall::StubFuture;

                #[test]
                fn ensure_linked() {
                    lucet_runtime::lucet_internal_ensure_linked();
                }

                #[test]
                fn load_module() {
                    let _module =
                        test_module_c("async_hostcall", "hostcall_block_on.c").expect("build and load module");
                }

                #[test]
                fn hostcall_yield() {
                    let module = test_module_c("async_hostcall", "hostcall_block_on.c")
                        .expect("module compiled and loaded");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
                        .expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.run_start().expect("start section runs");

                    let correct_run_res =
                        futures_executor::block_on(
                            inst.run_async(
                                "main",
                                &[0u32.into(), 0i32.into()]
                            )
                            // Run with bounded execution to test its interaction with block_on
                            .bound_inst_count(1)
                        );
                    match correct_run_res {
                        Ok(_) => {} // expected - UntypedRetVal is (), so no reason to inspect value
                        _ => panic!(
                            "run_async main should return successfully, got {:?}",
                            correct_run_res
                        ),
                    }

                    let incorrect_run_res = inst.run("main", &[0u32.into(), 0i32.into()]);
                    match incorrect_run_res {
                        Err(Error::RuntimeTerminated(term)) => {
                            assert_eq!(term, TerminationDetails::BlockOnNeedsAsync);
                        }
                        _ => panic!(
                            "inst.run should fail because its not an async context, got {:?}",
                            incorrect_run_res
                        ),
                    }
                    inst.reset().expect("can reset instance");

                    let correct_run_res_2 =
                        futures_executor::block_on(
                            inst.run_async(
                                "main",
                                &[0u32.into(), 0i32.into()]
                            ).bound_inst_count(1));
                    match correct_run_res_2 {
                        Ok(_) => {} // expected
                        _ => panic!(
                            "second run_async main should return successfully, got {:?}",
                            correct_run_res_2
                        ),
                    }

                    let correct_run_res_3 =
                        futures_executor::block_on(
                            inst.run_async(
                                "yielding",
                                &[]
                            ).bound_inst_count(10));

                    match correct_run_res_3 {
                        Ok(_) => {} // expected
                        _ => panic!(
                            "run_async yielding should return successfully, got {:?}",
                            correct_run_res_3
                        ),
                    }
                }


                #[test]
                fn yield_from_within_future() {
                    let module = test_module_c("async_hostcall", "hostcall_block_on.c")
                        .expect("module compiled and loaded");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
                        .expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.run_start().expect("start section runs");

                    let manual_future = StubFuture::new();

                    inst.insert_embed_ctx(Some(manual_future.clone()));

                    let run_res =
                        futures_executor::block_on(
                            inst.run_async(
                                "manual_future",
                                &[]
                            ));
                    
                    if let Ok(RunResult::Yielded(_)) = run_res { /* expected */ } else { panic!("did not yield"); } 

                    // The loop within try_block_on polled the future returned by await_manual_future,
                    // and the waker that will be passed to poll `manuall_future` is from the old
                    // executor.
                    //
                    // However, we yielded from the guest prior to polling manual_future, so we
                    // need to spawn a thread that will wake manual_future _after_ it has been polled.
                    std::thread::spawn(move || {
                        // the instance some time, so that it polls and blocks on manual_future
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        // wake the future manually. this will force us to miss the wakeup
                        manual_future.make_ready();
                    });

                    let run_res = futures_executor::block_on(inst.resume_async());
                    
                    if let Ok(RunResult::Returned(_)) = run_res { /* expected */ } else { panic!("did not return"); } 
                }
            }
        )*
    };
}
