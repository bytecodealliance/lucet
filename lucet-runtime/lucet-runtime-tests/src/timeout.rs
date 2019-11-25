use crate::helpers::{MockExportBuilder, MockModuleBuilder};
use lucet_module::FunctionPointer;
use lucet_runtime_internals::module::Module;
use lucet_runtime_internals::vmctx::lucet_vmctx;
use std::sync::Arc;

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

#[macro_export]
macro_rules! timeout_tests {
    ( $TestRegion:path ) => {
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, lucet_hostcall_terminate, DlModule, Error, FaultDetails, Instance,
            KillError, KillSuccess, Limits, Region, RunResult, SignalBehavior, TerminationDetails,
            TrapCode, YieldedVal,
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
        use $crate::timeout::mock_timeout_module;

        #[lucet_hostcall]
        #[no_mangle]
        pub fn slow_hostcall(vmctx: &mut Vmctx) -> bool {
            // make a window of time so we can timeout in a hostcall
            thread::sleep(Duration::from_millis(200));
            true
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn yielding_hostcall(vmctx: &mut Vmctx) {
            vmctx.yield_();
        }

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

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
            });

            match inst.run("infinite_loop", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {
                    // this is what we want to see
                }
                res => panic!("unexpected result: {:?}", res),
            }

            // after a timeout, can reset and run a normal function
            inst.reset().expect("instance resets");

            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_before_guest() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let kill_switch = inst.kill_switch();
            assert_eq!(kill_switch.terminate(), Err(KillError::NotTerminable));

            // not being terminable, the instance still runs and is unaffected
            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_after_guest() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("do_nothing", &[]) {
                Ok(_) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            let kill_switch = inst.kill_switch();
            assert_eq!(kill_switch.terminate(), Err(KillError::NotTerminable));

            // after a timeout, can reset and run a normal function
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

            match inst.run("main", &[0u32.into(), 0u32.into()]) {
                Err(Error::RuntimeFault(details)) => {
                    assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
                }
                res => panic!("unexpected result: {:?}", res),
            }

            let kill_switch = inst.kill_switch();
            assert_eq!(kill_switch.terminate(), Err(KillError::NotTerminable));

            // after a timeout, can reset and run a normal function
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

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Pending));
            });

            match inst.run("run_slow_hostcall", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // after a timeout, can reset and run a normal function
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

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Pending));
            });

            match inst.run("run_yielding_hostcall", &[]) {
                Ok(RunResult::Yielded(EmptyYieldVal)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            println!("waiting......");

            // wait for the timeout to expire
            thread::sleep(Duration::from_millis(200));

            match inst.resume() {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => panic!("unexpected result: {:?}", res),
            }

            // after a timeout, can reset and run a normal function
            inst.reset().expect("instance resets");

            run_onetwothree(&mut inst);
        }

        #[test]
        fn timeout_killswitch_reuse() {
            let module = test_module_c("timeout", "inf_loop.c").expect("build and load module");

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let kill_switch = inst.kill_switch();

            let t = thread::spawn(move || {
                assert!(kill_switch.terminate().is_err()); // fails too soon
                thread::sleep(Duration::from_millis(100));
                assert!(kill_switch.terminate().is_ok()); // works
                thread::sleep(Duration::from_millis(100));
                assert!(kill_switch.terminate().is_err()); // fails too soon
            });

            thread::sleep(Duration::from_millis(10));

            match inst.run("main", &[0u32.into(), 0u32.into()]) {
                // the result we're expecting - the guest has been terminated!
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {}
                res => {
                    panic!("unexpected result: {:?}", res);
                }
            };

            t.join().unwrap();
        }

        #[test]
        fn double_timeout() {
            let module = mock_timeout_module();
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let kill_switch = inst.kill_switch();
            let second_kill_switch = inst.kill_switch();

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
            });

            thread::spawn(move || {
                thread::sleep(Duration::from_millis(200));
                assert_eq!(
                    second_kill_switch.terminate(),
                    Err(KillError::NotTerminable)
                );
            });

            match inst.run("infinite_loop", &[]) {
                Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {
                    // this is what we want to see
                }
                res => panic!("unexpected result: {:?}", res),
            }

            // after a timeout, can reset and run a normal function
            inst.reset().expect("instance resets");

            run_onetwothree(&mut inst);
        }
    };
}
