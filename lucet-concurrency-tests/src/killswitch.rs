//! `KillSwitch` has documentation about the correctness of its various edge cases as part of
//! Lucet's wider documentation, see this section of the book for more:
//! https://github.com/bytecodealliance/lucet/blob/main/docs/src/lucet-runtime/killswitch.md#implementation-complexities-when-you-have-a-scheduler-full-of-demons
//!
//! This module contains actual test cases exercising these conditions where possible. Phrasing
//! from the above-mentioned section are re-used below in describing test cases. New tests should
//! have corresponding cases in the Lucet `KillSwitch` chapter!
use lucet_runtime::vmctx::Vmctx;
use lucet_runtime::{lucet_hostcall, Instance};
use std::sync::Arc;

use lucet_module::FunctionPointer;
use lucet_runtime_internals::lock_testpoints::Syncpoint;
use lucet_runtime_internals::module::Module;
use lucet_runtime_internals::module::{MockExportBuilder, MockModuleBuilder};
use lucet_runtime_internals::vmctx::lucet_vmctx;

pub static mut ENTERING_GUEST: Option<Syncpoint> = None;

/// A convenience wrapper around running our mock timeout module's `onetwothree` function,
/// and asserting that it returned the expected result.
pub fn run_onetwothree(inst: &mut Instance) {
    let retval = inst
        .run("onetwothree", &[])
        .expect("instance runs")
        .unwrap_returned();
    assert_eq!(libc::c_int::from(retval), 123);
}

/// Construct a Lucet module with surface area for testing all kinds of KillSwitch termination
/// cases.
///
/// This includes:
/// * Normal guest execution
/// * Hostcall execution
/// * Yield/resume
/// * Guest fault
/// * Hostcall fault
pub fn mock_killswitch_module() -> Arc<dyn Module> {
    extern "C" fn onetwothree(_vmctx: *const lucet_vmctx) -> std::os::raw::c_int {
        123
    }

    extern "C" fn run_guest(_vmctx: *const lucet_vmctx) {
        unsafe {
            // ENTERING_GUEST is only populated if the test requires this syncpoint be checked.
            if let Some(entering_guest) = ENTERING_GUEST.as_ref() {
                entering_guest.check();
            }
        }
    }

    extern "C" fn infinite_loop(_vmctx: *const lucet_vmctx) {
        unsafe {
            // ENTERING_GUEST is only populated if the test requires this syncpoint be checked.
            if let Some(entering_guest) = ENTERING_GUEST.as_ref() {
                entering_guest.check();
            }
        }
        loop {}
    }

    extern "C" fn fatal(vmctx: *const lucet_vmctx) {
        extern "C" {
            fn lucet_vmctx_get_heap(vmctx: *const lucet_vmctx) -> *mut u8;
        }

        unsafe {
            let heap_base = lucet_vmctx_get_heap(vmctx);

            // Using the default limits, each instance as of this writing takes up 0x200026000 bytes
            // worth of virtual address space. We want to access a point beyond all the instances,
            // so that memory is unmapped. We assume no more than 16 instances are mapped
            // concurrently. This may change as the library, test configuration, linker, phase of
            // moon, etc change, but for now it works.
            *heap_base.offset(0x0002_0002_6000 * 16) = 0;
        }
    }

    extern "C" fn hit_sigstack_guard_page(vmctx: *const lucet_vmctx) {
        extern "C" {
            fn lucet_vmctx_get_globals(vmctx: *const lucet_vmctx) -> *mut u8;
        }

        unsafe {
            let globals_base = lucet_vmctx_get_globals(vmctx);

            // Using the default limits, the globals are a page; try to write just off the end
            *globals_base.offset(0x1000) = 0;
        }
    }

    extern "C" fn do_nothing(_vmctx: *const lucet_vmctx) -> () {}

    extern "C" fn run_hostcall(vmctx: *const lucet_vmctx) -> bool {
        extern "C" {
            fn real_hostcall(vmctx: *const lucet_vmctx) -> bool;
        }
        unsafe { real_hostcall(vmctx) }
    }

    extern "C" fn run_yielding_hostcall(vmctx: *const lucet_vmctx) -> () {
        extern "C" {
            fn yielding_hostcall(vmctx: *const lucet_vmctx) -> ();
        }
        unsafe { yielding_hostcall(vmctx) }
    }

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "onetwothree",
            FunctionPointer::from_usize(onetwothree as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "infinite_loop",
            FunctionPointer::from_usize(infinite_loop as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "run_guest",
            FunctionPointer::from_usize(run_guest as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "do_nothing",
            FunctionPointer::from_usize(do_nothing as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "run_hostcall",
            FunctionPointer::from_usize(run_hostcall as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "run_yielding_hostcall",
            FunctionPointer::from_usize(run_yielding_hostcall as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "fatal",
            FunctionPointer::from_usize(fatal as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "hit_sigstack_guard_page",
            FunctionPointer::from_usize(hit_sigstack_guard_page as usize),
        ))
        .build()
}

/// This test hostcall just needs to exist so that we can call it. `LockTestpoints` provide points
/// for us to ensure that tests that need to happen inside a hostcall, will happen inside a
/// hostcall.
#[lucet_hostcall]
#[no_mangle]
pub fn real_hostcall(_vmctx: &Vmctx) -> bool {
    true
}

/// This test hostcall will immediately yield. This is used to test termination of a
/// yielded instance.
#[lucet_hostcall]
#[no_mangle]
pub fn yielding_hostcall(vmctx: &Vmctx) {
    vmctx.yield_();
}

#[macro_export]
macro_rules! killswitch_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::{
            lucet_hostcall, Error, Instance, InstanceHandle, KillError, KillSuccess, Limits,
            Region, RegionCreate, RunResult, TerminationDetails, TrapCode,
        };
        use lucet_runtime_internals::lock_testpoints::{SyncWaiter, Syncpoint};
        use lucet_runtime_tests::build::test_module_c;
        use lucet_runtime_tests::helpers::test_ex;
        use lucet_runtime_tests::helpers::test_nonex;
        use std::thread;
        use $TestRegion as TestRegion;
        use $crate::killswitch::mock_killswitch_module;
        use $crate::killswitch::run_onetwothree;
        use $crate::killswitch::ENTERING_GUEST;

        pub fn test_c_with_instrumented_guest_entry<F, R>(dir: &str, cfile: &str, f: F) -> R
        where
            F: FnOnce(InstanceHandle) -> R,
        {
            test_ex(|| {
                unsafe {
                    ENTERING_GUEST = Some(Syncpoint::new());
                }
                let module = test_module_c(dir, cfile).expect("build and load module");
                let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
                    .expect("region can be created");

                let inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                f(inst)
            })
        }

        /// Run a test with excusive access to process-wide resources.
        ///
        /// This function must wrap tests that use `ENTERING_GUEST` or rely on having a specific signal
        /// handler installed.
        pub fn test_exclusive_instance_with_instrumented_guest_entry<F, R>(f: F) -> R
        where
            F: FnOnce(InstanceHandle) -> R,
        {
            test_ex(|| {
                unsafe {
                    ENTERING_GUEST = Some(Syncpoint::new());
                }
                let module = mock_killswitch_module();
                let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
                    .expect("region can be created");

                let inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                f(inst)
            })
        }

        /// Run a test with shared access to process-wide resources.
        ///
        /// This wrapper can be used to execute tests that do not reference `ENTERING_GUEST` or rely on
        /// signal handler details. This is preferential to allow tests to run in parallel.
        pub fn test_instance_with_instrumented_guest_entry<F, R>(f: F) -> R
        where
            F: FnOnce(InstanceHandle) -> R,
        {
            test_nonex(|| {
                let module = mock_killswitch_module();
                let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
                    .expect("region can be created");

                let inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                f(inst)
            })
        }

        // Test that termination in a guest works without signalling the embedder.
        //
        // This corresponds to the documentation's State B -> State E transition due to guest
        // fault/termination.
        #[test]
        fn terminate_in_guest() {
            test_exclusive_instance_with_instrumented_guest_entry(|mut inst| {
                let in_guest = unsafe { ENTERING_GUEST.as_ref().unwrap().wait_at() };

                let (kill_switch, outstanding_killswitch) =
                    (inst.kill_switch(), inst.kill_switch());

                let t = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || {
                        match inst.run("infinite_loop", &[]) {
                            Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {
                                // this is what we want!
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        // A freshly acquired kill switch can cancel the next execution.
                        // Test here rather than the outer test body because this closure moves `inst`.
                        assert_eq!(inst.kill_switch().terminate(), Ok(KillSuccess::Cancelled));
                    })
                    .expect("can spawn a thread");

                let terminator = in_guest.wait_and_then(move || {
                    thread::spawn(move || {
                        assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
                    })
                });

                t.join().unwrap();
                terminator.join().unwrap();

                // Outstanding kill switches fail, because the kill state was reset.
                assert_eq!(outstanding_killswitch.terminate(), Err(KillError::Invalid));
            })
        }

        // Test that termination while entering a guest works without signalling the embedder.
        //
        // This corresponds to a race during the documentation's State A -> State B transition.
        #[test]
        fn terminate_entering_guest() {
            let test_entering_guest_before_domain_change: fn(&Instance) -> SyncWaiter =
                |inst: &Instance| -> SyncWaiter {
                    inst.lock_testpoints
                        .instance_entering_guest_before_domain_change
                        .wait_at()
                };
            let test_entering_guest_after_domain_change: fn(&Instance) -> SyncWaiter =
                |inst: &Instance| -> SyncWaiter {
                    inst.lock_testpoints
                        .instance_entering_guest_after_domain_change
                        .wait_at()
                };

            for (i, racepoint_builder) in [
                test_entering_guest_before_domain_change,
                test_entering_guest_after_domain_change,
            ]
            .iter()
            .enumerate()
            {
                println!("testing racepoint {}", i);
                test_exclusive_instance_with_instrumented_guest_entry(|mut inst| {
                    let kill_switch = inst.kill_switch();
                    let racepoint = racepoint_builder(&inst);

                    let guest = thread::Builder::new()
                        .name("guest".to_owned())
                        .spawn(move || match inst.run("run_guest", &[]) {
                            Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                            res => panic!("unexpectd result: {:?}", res),
                        })
                        .expect("can spawn thread to run guest");

                    racepoint.wait_and_then(|| {
                        kill_switch.terminate().expect("can terminate in guest");
                    });

                    guest.join().expect("guest exits without panic");
                })
            }
        }

        // Test a termination that completes right before `exit_guest_region` takes ownership of termination.
        //
        // This corresponds to a race during the documentation's State B -> State E "due to normal exit"
        // transition.
        //
        // This test of race with a B->E transition is split into multiple test functions because there are
        // multiple kinds of races to test with different observed behaviors.
        #[test]
        fn terminate_exiting_guest_before_domain_change() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();
                let racepoint = inst
                    .lock_testpoints
                    .instance_exiting_guest_before_acquiring_terminable
                    .wait_at();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || match inst.run("run_guest", &[]) {
                        Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                        res => panic!("unexpectd result: {:?}", res),
                    })
                    .expect("can spawn thread to run guest");

                racepoint.wait_and_then(|| {
                    kill_switch
                        .terminate()
                        .expect("can terminate before exiting guest");
                });

                guest.join().expect("guest exits without panic");
            })
        }

        // Test a termination that completes right after `exit_guest_region` finishes setting `KillState`
        // as an exited guest, but before actually returning to the host.
        //
        // This corresponds to a race during the documentation's State B -> State E "due to normal exit"
        // transition.
        //
        // This test of race with a B->E transition is split into multiple test functions because there are
        // multiple kinds of races to test with different observed behaviors.
        #[test]
        fn terminate_exiting_guest_after_domain_change() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();
                let racepoint = inst
                    .lock_testpoints
                    .instance_exiting_guest_after_domain_change
                    .wait_at();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || {
                        match inst.run("run_guest", &[]) {
                            Ok(RunResult::Returned(_)) => {
                                // We intentionally have `KillState` lose this race, so the guest should
                                // return normally.
                            }
                            res => panic!("unexpectd result: {:?}", res),
                        }
                    })
                    .expect("can spawn thread to run guest");

                racepoint.wait_and_then(|| {
                    // We are terminating immediately after disabling termination, but the disabled
                    // `KillState` has not yet been dropped. We can determine this instance is not
                    // terminable, but has a valid `KillState` reference.
                    assert_eq!(kill_switch.terminate(), Err(KillError::NotTerminable));
                });

                guest.join().expect("guest exits without panic");
            })
        }

        // Test a termination begins before `exit_guest_region`, so the guest checks `terminable` during an
        // in-flight termination.
        //
        // We want this specific sequence of events:
        // * guest reaches exit_guest_region
        // * killswitch fires, acquiring `terminable`
        // * guest observes `terminable` is false, so it must wait for termination
        // * killswitch terminates and completes while guest is waiting
        //
        // This corresponds to a race during the documentation's State B -> State E "due to normal exit"
        // transition.
        //
        // This test of race with a B->E transition is split into multiple test functions because there are
        // multiple kinds of races to test with different observed behaviors.
        #[test]
        fn terminate_exiting_guest_during_terminable_check() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();
                let exit_guest_region = inst
                    .lock_testpoints
                    .instance_exiting_guest_before_acquiring_terminable
                    .wait_at();
                let guest_wait_for_signal = inst
                    .lock_testpoints
                    .instance_exiting_guest_without_terminable
                    .wait_at();
                let killswitch_acquired_termination = inst
                    .lock_testpoints
                    .kill_switch_after_acquiring_termination
                    .wait_at();
                let killswitch_guest_signal = inst
                    .lock_testpoints
                    .kill_switch_before_guest_alarm
                    .wait_at();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || match inst.run("run_guest", &[]) {
                        Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                        res => panic!("unexpectd result: {:?}", res),
                    })
                    .expect("can spawn thread to run guest");

                // When the instance has reached `exit_guest_region`, start a thread to terminate the
                // guest, then wait for it to acquire `terminable`. This is all to ensure that `terminable`
                // is false by the time we allow `exit_guest_region` to proceed.
                let killswitch_thread = exit_guest_region.wait_and_then(|| {
                    let new_thread = thread::Builder::new()
                        .name("killswitch".to_owned())
                        .spawn(move || {
                            kill_switch
                                .terminate()
                                .expect("can terminate before exiting guest")
                        })
                        .expect("can spawn killswitch thread");
                    killswitch_acquired_termination.wait();
                    new_thread
                });

                // When the `KillSwitch` is about to signal, make sure the guest has actually checked it
                // cannot exit. Once it has, let the `KillSwitch` terminate the guest and complete our
                // test!
                killswitch_guest_signal.wait_and_then(|| {
                    guest_wait_for_signal.wait();
                });

                killswitch_thread
                    .join()
                    .expect("killswitch completes without panic");
                guest.join().expect("guest exits without panic");
            })
        }

        // If we terminate in the signal handler, but before termination has been disabled, a
        // signal will be sent to the guest. Lucet must correctly handle this case, lest the sigalrm be
        // delivered to disastrous effect to the host.
        //
        // This corresponds to a race during the documentation's State B -> State E "guest faults
        // or is terminated" transition.
        #[test]
        fn terminate_during_guest_fault() {
            test_c_with_instrumented_guest_entry("timeout", "fault.c", |mut inst| {
                let kill_switch = inst.kill_switch();

                // *Before* termination is critical, since afterward the `KillSwitch` we test with will
                // just take no action.
                let unfortunate_time_to_terminate = inst
                    .lock_testpoints
                    .signal_handler_before_disabling_termination
                    .wait_at();
                // Wait for the guest to reach a point we reaaallly don't want to signal at - somewhere in
                // the signal handler.
                let exiting_signal_handler = inst
                    .lock_testpoints
                    .signal_handler_before_returning
                    .wait_at();
                // Finally, we need to know when we're ready to signal to ensure it races with.
                let killswitch_send_signal =
                    inst.lock_testpoints.kill_switch_after_guest_alarm.wait_at();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || {
                        match inst.run("main", &[0u32.into(), 0u32.into()]) {
                            Err(Error::RuntimeFault(details)) => {
                                assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        // Check that we can reset the instance and run a normal function.
                        inst.reset().expect("instance resets");
                        run_onetwothree(&mut inst);
                    })
                    .expect("can spawn guest thread");

                let termination_thread = unfortunate_time_to_terminate.wait_and_then(|| {
                    let thread = thread::Builder::new()
                        .name("killswitch".to_owned())
                        .spawn(move || {
                            assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
                        })
                        .expect("can spawn killswitch thread");
                    killswitch_send_signal.wait();
                    thread
                });

                // Get ready to signal...
                // and be sure that we haven't exited the signal handler until afterward
                exiting_signal_handler.wait();

                guest.join().expect("guest exits without panic");
                termination_thread
                    .join()
                    .expect("termination completes without panic");
            })
        }

        // Variant of the above where for scheduler reasons `terminable` and
        // `execution_domain.lock()` happen on different sides of an instance descheduling.
        //
        // This corresponds to a race during the documentation's State B -> State E "guest faults
        // or is terminated" transition.
        //
        // Specifically, we want:
        // * signal handler fires, handling a guest fault
        // * timeout fires, acquiring terminable
        // * signal handler completes, locking in deschedule to serialize pending KillSwitch
        // * KillSwitch is rescheduled, then fires
        //
        // And for all of this to complete without error!
        #[test]
        fn terminate_during_guest_fault_racing_deschedule() {
            test_c_with_instrumented_guest_entry("timeout", "fault.c", |mut inst| {
                let kill_switch = inst.kill_switch();

                // *before* termination is critical, since afterward the `KillSwitch` we test with will
                // just take no action.
                let unfortunate_time_to_terminate = inst
                    .lock_testpoints
                    .signal_handler_before_disabling_termination
                    .wait_at();
                // we need to let the instance deschedule before our KillSwitch takes
                // `execution_domain`.
                let killswitch_acquire_termination = inst
                    .lock_testpoints
                    .kill_switch_after_acquiring_termination
                    .wait_at();
                // and the entire test revolves around KillSwitch taking effect after
                // `CURRENT_INSTANCE` is cleared!
                let current_instance_cleared = inst
                    .lock_testpoints
                    .instance_after_clearing_current_instance
                    .wait_at();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || {
                        match inst.run("main", &[0u32.into(), 0u32.into()]) {
                            Err(Error::RuntimeFault(details)) => {
                                assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        // Check that we can reset the instance and run a normal function.
                        inst.reset().expect("instance resets");
                        run_onetwothree(&mut inst);
                    })
                    .expect("can spawn guest thread");

                let (termination_thread, killswitch_before_domain) = unfortunate_time_to_terminate
                    .wait_and_then(|| {
                        let ks_thread = thread::Builder::new()
                            .name("killswitch".to_owned())
                            .spawn(move || {
                                assert_eq!(kill_switch.terminate(), Err(KillError::NotTerminable));
                            })
                            .expect("can spawn killswitch thread");

                        // Pause the KillSwitch thread right before it acquires `execution_domain`
                        let killswitch_before_domain = killswitch_acquire_termination.pause();

                        (ks_thread, killswitch_before_domain)
                    });

                // `execution_domain` is not held, so instance descheduling will complete promptly.
                current_instance_cleared.wait();

                // Resume `KillSwitch`, which will acquire `execution_domain` and terminate.
                killswitch_before_domain.resume();

                guest.join().expect("guest exits without panic");
                termination_thread
                    .join()
                    .expect("termination completes without panic");
            })
        }

        // This doesn't doesn't correspond to any state change in the documentation because it should have
        // no effect. The guest is in State E before, and should remain in State E after.
        #[test]
        fn terminate_after_guest_fault() {
            test_c_with_instrumented_guest_entry("timeout", "fault.c", |mut inst| {
                let kill_switch = inst.kill_switch();

                match inst.run("main", &[0u32.into(), 0u32.into()]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
                    }
                    res => panic!("unexpected result: {:?}", res),
                }

                // An instance that has faulted is not terminable.
                assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));

                // Check that we can reset the instance and run a normal function.
                inst.reset().expect("instance resets");
                run_onetwothree(&mut inst);
            })
        }

        // This corresponds to the documentation's State C -> State E "terminate in hostcall observed"
        // transition.
        #[test]
        fn terminate_in_hostcall() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();
                let in_hostcall = inst
                    .lock_testpoints
                    .instance_exiting_hostcall_before_domain_change
                    .wait_at();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || match inst.run("run_hostcall", &[]) {
                        Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                        res => panic!("unexpectd result: {:?}", res),
                    })
                    .expect("can spawn thread to run guest");

                in_hostcall.wait_and_then(|| {
                    kill_switch.terminate().expect("can terminate in hostcall");
                });

                guest.join().expect("guest exits without panic");
            })
        }

        // This corresponds to a race during the documentation's State C -> State B "hostcall returns
        // normally" transition. On either side of this transition, the guest should terminate.
        #[test]
        fn terminate_exiting_hostcall() {
            let test_exiting_hostcall_before_domain_change: fn(&Instance) -> SyncWaiter =
                |inst: &Instance| -> SyncWaiter {
                    inst.lock_testpoints
                        .instance_exiting_hostcall_before_domain_change
                        .wait_at()
                };
            let test_exiting_hostcall_after_domain_change: fn(&Instance) -> SyncWaiter =
                |inst: &Instance| -> SyncWaiter {
                    inst.lock_testpoints
                        .instance_exiting_hostcall_after_domain_change
                        .wait_at()
                };

            for (i, racepoint_builder) in [
                test_exiting_hostcall_before_domain_change,
                test_exiting_hostcall_after_domain_change,
            ]
            .iter()
            .enumerate()
            {
                println!("testing racepoint {}", i);
                test_instance_with_instrumented_guest_entry(|mut inst| {
                    let kill_switch = inst.kill_switch();
                    let racepoint = racepoint_builder(&inst);

                    let guest = thread::Builder::new()
                        .name("guest".to_owned())
                        .spawn(move || match inst.run("run_hostcall", &[]) {
                            Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                            res => panic!("unexpectd result: {:?}", res),
                        })
                        .expect("can spawn thread to run guest");

                    racepoint.wait_and_then(|| {
                        kill_switch.terminate().expect("can terminate in hostcall");
                    });

                    guest.join().expect("guest exits without panic");
                })
            }
        }

        // This corresponds to a race during the documentation's State B -> State C "guest makes hostcall"
        // transition. On either side of this transition, the guest should terminate.
        #[test]
        fn terminate_entering_hostcall() {
            let test_entering_hostcall_before_domain_change: fn(&Instance) -> SyncWaiter =
                |inst: &Instance| -> SyncWaiter {
                    inst.lock_testpoints
                        .instance_entering_hostcall_before_domain_change
                        .wait_at()
                };
            let test_entering_hostcall_after_domain_change: fn(&Instance) -> SyncWaiter =
                |inst: &Instance| -> SyncWaiter {
                    inst.lock_testpoints
                        .instance_entering_hostcall_after_domain_change
                        .wait_at()
                };

            for (i, racepoint_builder) in [
                test_entering_hostcall_before_domain_change,
                test_entering_hostcall_after_domain_change,
            ]
            .iter()
            .enumerate()
            {
                println!("testing racepoint {}", i);
                test_instance_with_instrumented_guest_entry(|mut inst| {
                    let kill_switch = inst.kill_switch();
                    let racepoint = racepoint_builder(&inst);

                    let guest = thread::Builder::new()
                        .name("guest".to_owned())
                        .spawn(move || match inst.run("run_hostcall", &[]) {
                            Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                            res => panic!("unexpectd result: {:?}", res),
                        })
                        .expect("can spawn thread to run guest");

                    racepoint.wait_and_then(|| {
                        kill_switch.terminate().expect("can terminate in hostcall");
                    });

                    guest.join().expect("guest exits without panic");
                })
            }
        }

        /// This test ensures that we see an `Invalid` kill error if we are attempting to terminate an
        /// instance that has since been dropped. It does not correspond to any state in the documentation
        /// because the documentation only concerns live instances, and terminating a dropped instance
        /// should have no effect either way.
        #[test]
        fn terminate_after_guest_drop() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();
                std::mem::drop(inst);
                assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));
            });
        }

        // This doesn't doesn't correspond to any state change in the documentation because it should have
        // no effect. The guest is in State E before, and should remain in State E after.
        #[test]
        fn timeout_after_guest_runs() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();

                // The killswitch will fail if the instance has already finished running.
                match inst.run("do_nothing", &[]) {
                    Ok(_) => {}
                    res => panic!("unexpected result: {:?}", res),
                }

                // If we try to terminate after the instance ran, the kill switch will fail - the
                // function we called is no longer running - and the the instance will run normally the
                // next time around.
                assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));
                match inst.run("do_nothing", &[]) {
                    Ok(_) => {}
                    res => panic!("unexpected result: {:?}", res),
                }

                // Check that we can reset the instance and run a normal function.
                inst.reset().expect("instance resets");
                run_onetwothree(&mut inst);
            });
        }

        // This corresponds to the documentation's State C -> State E "terminate in hostcall observed"
        // transition because instance yielding behaves the same as making a hostcall.
        #[test]
        fn timeout_while_yielded() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();

                // Start the instance, running a function that will yield.
                match inst.run("run_yielding_hostcall", &[]) {
                    Ok(RunResult::Yielded(val)) => {
                        assert!(val.is_none());
                    }
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
            });
        }

        // Terminating an instance twice works, does not explode, and the second termination is an `Err`
        // because the instance is no longer terminable. This does not correspond to any part of
        // `KillSwitch` because the instance is terminated and the second termination should have no
        // additional effect.
        #[test]
        fn double_terminate() {
            test_exclusive_instance_with_instrumented_guest_entry(|mut inst| {
                let in_guest = unsafe { ENTERING_GUEST.as_ref().unwrap().wait_at() };

                let guest_exit = Syncpoint::new();
                let guest_exit_testpoint = guest_exit.wait_at();

                let kill_switch = inst.kill_switch();
                let second_kill_switch = inst.kill_switch();

                let guest = thread::Builder::new()
                    .name("guest".to_owned())
                    .spawn(move || {
                        // Start the instance, which will return an error having been remotely terminated.
                        match inst.run("infinite_loop", &[]) {
                            Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                            res => panic!("unexpected result: {:?}", res),
                        }

                        guest_exit.check();

                        // Check that we can reset the instance and run a function.
                        inst.reset().expect("instance resets");
                        run_onetwothree(&mut inst);

                        // Finally, check that a freshly acquired kill switch can cancel the next execution.
                        assert_eq!(inst.kill_switch().terminate(), Ok(KillSuccess::Cancelled));
                    })
                    .expect("can spawn the guest thread");

                // Wait to actually reach the guest.
                let ks1 = in_guest.wait_and_then(move || {
                    thread::Builder::new()
                        .name("killswitch_1".to_owned())
                        .spawn(move || {
                            assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
                        })
                        .expect("can spawn killswitch 1 termination thread")
                });

                ks1.join().expect("killswitch_1 did not panic");

                // Allow the instance to reset and run a new function after termination.
                guest_exit_testpoint.wait_and_then(|| {
                    // At this point the first `KillSwitch` has completed terminating the instance. Now try
                    // again and make sure there's no boom.
                    assert_eq!(second_kill_switch.terminate(), Err(KillError::Invalid));
                });

                // And after the instance successfully runs a test function, it exits without error.
                guest.join().expect("guest stops running");
            })
        }

        // This corresponds to the documentation's State A -> State D "terminate before execution"
        // transition.
        #[test]
        fn timeout_before_guest_runs() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
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
            });
        }

        /// This test ensures that we see a more informative kill error than `NotTerminable` when
        /// attempting to terminate an instance that has been reset since issuing a kill switch. It does
        /// not correspond to any state in the documentation because the documentation only concerns live
        /// instances, and terminating a dropped instance should have no effect either way.
        #[test]
        fn timeout_after_guest_reset() {
            test_instance_with_instrumented_guest_entry(|mut inst| {
                let kill_switch = inst.kill_switch();
                inst.reset().expect("instance resets");
                assert_eq!(kill_switch.terminate(), Err(KillError::Invalid));
                run_onetwothree(&mut inst);
            });
        }
    };
}
