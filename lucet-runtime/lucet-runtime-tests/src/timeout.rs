//! Termination tests.
//!
//! This macro tests that instances within some kind of memory region can be terminated properly
//! using a remote kill switch. See
//! [`KillSwitch::terminate`](struct.KillSwitch.html#method.terminate) for more information.
#[macro_export]
macro_rules! timeout_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, lucet_hostcall_terminate, DlModule, Error, FaultDetails, Instance,
            KillError, KillSuccess, Limits, Module, Region, RunResult, SignalBehavior,
            TerminationDetails, TrapCode, YieldedVal,
        };
        use nix::sys::mman::{mmap, MapFlags, ProtFlags};
        use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{fork, ForkResult};
        use std::ptr;
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;
        use $TestRegion as TestRegion;
        use $crate::build::test_module_c;
        use $crate::helpers::{FunctionPointer, MockExportBuilder, MockModuleBuilder};

        /// Return a mock module so that we can test termination behavior.
        ///
        /// See `lucet_runtime_internals::module::mock::MockModuleBuilder` for more information.
        pub fn mock_timeout_module() -> Arc<dyn Module> {
            extern "C" fn onetwothree(_vmctx: *mut lucet_vmctx) -> std::os::raw::c_int {
                123
            }

            extern "C" fn infinite_loop(_vmctx: *mut lucet_vmctx) -> () {
                loop {}
            }

            extern "C" fn do_nothing(_vmctx: *mut lucet_vmctx) -> () {}

            extern "C" fn run_slow_hostcall(vmctx: *mut lucet_vmctx) -> bool {
                extern "C" {
                    fn slow_hostcall(vmctx: *mut lucet_vmctx) -> bool;
                }
                unsafe { slow_hostcall(vmctx) }
            }

            extern "C" fn run_yielding_hostcall(vmctx: *mut lucet_vmctx) -> () {
                extern "C" {
                    fn yielding_hostcall(vmctx: *mut lucet_vmctx) -> ();
                }
                unsafe { yielding_hostcall(vmctx) }
            }

            MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "infinite_loop",
                    FunctionPointer::from_usize(infinite_loop as usize),
                ))
                .with_export_func(MockExportBuilder::new(
                    "do_nothing",
                    FunctionPointer::from_usize(do_nothing as usize),
                ))
                .with_export_func(MockExportBuilder::new(
                    "onetwothree",
                    FunctionPointer::from_usize(onetwothree as usize),
                ))
                .with_export_func(MockExportBuilder::new(
                    "run_slow_hostcall",
                    FunctionPointer::from_usize(run_slow_hostcall as usize),
                ))
                .with_export_func(MockExportBuilder::new(
                    "run_yielding_hostcall",
                    FunctionPointer::from_usize(run_yielding_hostcall as usize),
                ))
                .build()
        }

        /// This test hostcall will wait for 200 milliseconds before returning `true`.
        /// This is used to make a window of time so we can timeout inside of a hostcall.
        #[lucet_hostcall]
        #[no_mangle]
        pub fn slow_hostcall(vmctx: &mut Vmctx) -> bool {
            thread::sleep(Duration::from_millis(200));
            true
        }

        /// This test hostcall will immediately yield. This is used to test termination of a
        /// yielded instance.
        #[lucet_hostcall]
        #[no_mangle]
        pub fn yielding_hostcall(vmctx: &mut Vmctx) {
            vmctx.yield_();
        }

        /// A convenience wrapper around running our mock timeout module's `onetwothree` function,
        /// and asserting that it returned the expected result.
        fn run_onetwothree(inst: &mut Instance) {
            let retval = inst
                .run("onetwothree", &[])
                .expect("instance runs")
                .unwrap_returned();
            assert_eq!(libc::c_int::from(retval), 123);
        }

        #[test]
        fn timeout_in_guest() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();

            // Spawn a thread to terminate the instance after waiting for 100ms.
            let t = thread::Builder::new()
                .name("killswitch".to_owned())
                .spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                    assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
                })
                .expect("can spawn a thread");

            // Begin running the instance, which will be terminated remotely by the KillSwitch.
            match inst.run("infinite_loop", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {
                    // this is what we want to see
                }
                res => panic!("unexpected result: {:?}", res),
            }
            t.join().unwrap();

            // Another attempt to terminate the instance will fail.
            assert_eq!(
                inst.kill_switch().terminate(),
                Err(KillError::NotTerminable)
            );

            // Check that we can reset the instance and run a normal function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_before_guest_runs() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();

            // If terminated before running, the guest will be cancelled.
            assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Cancelled));

            // Another attempt to terminate the instance will fail.
            assert_eq!(
                inst.kill_switch().terminate(),
                Err(KillError::NotTerminable)
            );

            match inst.run("onetwothree", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // Check that we can reset the instance and run a normal function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_after_guest_runs() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();

            // The killswitch will fail if the instance has already finished running.
            match inst.run("do_nothing", &[]) {
                Ok(_) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // If we try to terminate after the instance ran, the kill switch will fail, and the
            // the instance will run normally the next time around.
            assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));
            match inst.run("do_nothing", &[]) {
                Ok(_) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // Check that we can reset the instance and run a normal function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_after_guest_fault() {
            let module = test_module_c("timeout", "fault.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();

            // Run the faulting guest.
            match inst.run("main", &[0u32.into(), 0u32.into()]) {
                Err(Error::RuntimeFault(details)) => {
                    assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
                }
                res => panic!("unexpected result: {:?}", res),
            }

            // An instance that has faulted is not terminable.
            assert_eq!(kill_switch.terminate(), Err(KillError::NotTerminable));

            // Check that we can reset the instance and run a normal function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_in_hostcall() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();

            // Spawn a thread to terminate the instance after waiting for 100ms.
            thread::Builder::new()
                .name("killswitch".to_owned())
                .spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                    assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Pending));
                })
                .expect("can spawn a thread");

            // Begin running the instance, which will be terminated remotely by the KillSwitch
            // while inside a hostcall. See `slow_hostcall` above for more information.
            match inst.run("run_slow_hostcall", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // Another attempt to terminate the instance will fail.
            assert_eq!(
                inst.kill_switch().terminate(),
                Err(KillError::NotTerminable)
            );

            // Check that we can reset the instance and run a normal function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_while_yielded() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();

            // Start the instance, running a function that will yield.
            match inst.run("run_yielding_hostcall", &[]) {
                Ok(RunResult::Yielded(EmptyYieldVal)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // A yielded instance can only be scheduled for termination.
            assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Pending));

            // A second attempt to terminate a yielded instance will fail.
            assert_eq!(
                inst.kill_switch().terminate(),
                Err(KillError::NotTerminable)
            );

            // Once resumed, the terminated instance will be terminated.
            match inst.resume() {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // Check that we can reset the instance and run a normal function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }

        /// This test ensures that we see a more informative kill error than `NotTerminable` when
        /// attempting to terminate an instance that has been reset since issuing a kill switch.
        #[test]
        fn timeout_after_guest_reset() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();
            inst.reset().expect("instance resets");
            assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));
            run_onetwothree(&mut inst);
        }

        /// This test ensures that we see an `Invalid` kill error if we are attempting to terminate
        /// an instance that has since been dropped.
        #[test]
        fn timeout_after_guest_drop() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            let kill_switch = inst.kill_switch();
            std::mem::drop(inst);
            assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));
        }

        #[test]
        fn double_timeout() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // Spawn a thread to terminate the instance after waiting for 100ms.
            let kill_switch = inst.kill_switch();
            let t1 = thread::Builder::new()
                .name("killswitch_1".to_owned())
                .spawn(move || {
                    thread::sleep(Duration::from_millis(100));
                    assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
                })
                .expect("can spawn a thread");

            // Spawn a thread to terminate the instance after waiting for 200ms.
            let second_kill_switch = inst.kill_switch();
            let t2 = thread::Builder::new()
                .name("killswitch_2".to_owned())
                .spawn(move || {
                    thread::sleep(Duration::from_millis(200));
                    assert_eq!(
                        second_kill_switch.terminate(),
                        Err(KillError::NotTerminable)
                    );
                })
                .expect("can spawn a thread");

            // Start the instance, which will return an error having been remotely terminated.
            match inst.run("infinite_loop", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // Explicitly check that each helper thread's assertions succeeded.
            t1.join().expect("killswitch_1 did not panic");
            t2.join().expect("killswitch_2 did not panic");

            // Check that we can reset the instance and run a function.
            inst.reset().expect("instance resets");
            run_onetwothree(&mut inst);
        }
    };
}
