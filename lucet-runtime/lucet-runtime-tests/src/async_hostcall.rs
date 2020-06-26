#[macro_export]
macro_rules! async_hostcall_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {

        use lucet_runtime::{vmctx::Vmctx, lucet_hostcall};

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_containing_block_on(vmctx: &Vmctx, value: u32) {
            let asynced_value = vmctx.block_on(async move { value });
            assert_eq!(asynced_value, value);
        }

        $(
            mod $region_id {
                use lucet_runtime::{DlModule, Error, Limits, Region, RegionCreate, TerminationDetails, RunResult};
                use std::sync::Arc;
                use $TestRegion as TestRegion;
                use $crate::build::test_module_c;

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
                    /// Dummy function with the same type signature as
                    /// [`tokio::task::block_in_place`][tokio].
                    ///
                    /// [tokio]: https://docs.rs/tokio/0.2.21/tokio/task/fn.block_in_place.html
                    fn block_in_place<F, R>(f: F) -> R
                    where
                        F: FnOnce() -> R,
                    {
                        f()
                    }

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
                                &[0u32.into(), 0i32.into()],
                                |f| block_in_place(f),
                            ));
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
                            assert_eq!(term, TerminationDetails::AwaitNeedsAsync);
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
                                &[0u32.into(), 0i32.into()],
                                |f| block_in_place(f),
                            ));
                    match correct_run_res_2 {
                        Ok(RunResult::Returned { .. }) => {} // expected
                        _ => panic!(
                            "second run_async main should return successfully, got {:?}",
                            correct_run_res_2
                        ),
                    }
                }

            }
        )*
    };
}
