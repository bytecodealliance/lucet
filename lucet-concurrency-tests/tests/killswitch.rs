use lucet_runtime::{lucet_hostcall, Error, Instance, InstanceHandle, Limits, KillError, KillSuccess, Region, TerminationDetails, TrapCode};
use lucet_runtime::vmctx::Vmctx;
use std::time::Duration;
use std::sync::Arc;
use std::thread;

use lucet_module::FunctionPointer;
use lucet_runtime::MmapRegion;
use lucet_runtime_internals::module::Module;
use lucet_runtime_internals::module::{MockExportBuilder, MockModuleBuilder};
use lucet_runtime_internals::vmctx::lucet_vmctx;
use lucet_runtime_internals::lock_testpoints::Syncpoint;
use lucet_runtime_tests::helpers::test_ex;
use lucet_runtime_tests::build::test_module_c;

static mut ENTERING_GUEST: Option<Syncpoint> = None;

/// A convenience wrapper around running our mock timeout module's `onetwothree` function,
/// and asserting that it returned the expected result.
fn run_onetwothree(inst: &mut Instance) {
    let retval = inst
        .run("onetwothree", &[])
        .expect("instance runs")
        .unwrap_returned();
    assert_eq!(libc::c_int::from(retval), 123);
}

pub fn mock_traps_module() -> Arc<dyn Module> {
    extern "C" fn onetwothree(_vmctx: *mut lucet_vmctx) -> std::os::raw::c_int {
        123
    }

    extern "C" fn infinite_loop(_vmctx: *mut lucet_vmctx) {
        unsafe {
            ENTERING_GUEST.as_ref().unwrap().check();
        }
        loop {}
    }

    extern "C" fn fatal(vmctx: *mut lucet_vmctx) {
        extern "C" {
            fn lucet_vmctx_get_heap(vmctx: *mut lucet_vmctx) -> *mut u8;
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

    extern "C" fn hit_sigstack_guard_page(vmctx: *mut lucet_vmctx) {
        extern "C" {
            fn lucet_vmctx_get_globals(vmctx: *mut lucet_vmctx) -> *mut u8;
        }

        unsafe {
            let globals_base = lucet_vmctx_get_globals(vmctx);

            // Using the default limits, the globals are a page; try to write just off the end
            *globals_base.offset(0x1000) = 0;
        }
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
            "onetwothree",
            FunctionPointer::from_usize(onetwothree as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "infinite_loop",
            FunctionPointer::from_usize(infinite_loop as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "do_nothing",
            FunctionPointer::from_usize(do_nothing as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "run_slow_hostcall",
            FunctionPointer::from_usize(run_slow_hostcall as usize),
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

/// This test hostcall will wait for 200 milliseconds before returning `true`.
/// This is used to make a window of time so we can timeout inside of a hostcall.
#[lucet_hostcall]
#[no_mangle]
pub fn slow_hostcall(_vmctx: &mut Vmctx) -> bool {
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

pub fn test_c_with_instrumented_guest_entry<F, R>(dir: &str, cfile: &str, f: F) -> R
where
    F: FnOnce(InstanceHandle) -> R,
{
    test_ex(|| {
        unsafe {
            ENTERING_GUEST = Some(Syncpoint::new());
        }
        let module = test_module_c(dir, cfile).expect("build and load module");
        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

        let inst = region
            .new_instance(module)
            .expect("instance can be created");

        f(inst)
    })
}

pub fn test_instance_with_instrumented_guest_entry<F, R>(f: F) -> R
where
    F: FnOnce(InstanceHandle) -> R,
{
    test_ex(|| {
        unsafe {
            ENTERING_GUEST = Some(Syncpoint::new());
        }
        let module = mock_traps_module();
        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");

        let inst = region
            .new_instance(module)
            .expect("instance can be created");

        f(inst)
    })
}

// Test that a timeout that occurs in a signal handler is handled cleanly without signalling the
// Lucet embedder.
#[test]
fn terminate_in_guest() {
    test_instance_with_instrumented_guest_entry(|mut inst| {
        let in_guest = unsafe { ENTERING_GUEST.as_ref().unwrap().wait_at() };

        let kill_switch = inst.kill_switch();

        let t = thread::Builder::new()
            .name("guest".to_owned())
            .spawn(move || {
                match inst.run("infinite_loop", &[]) {
                    Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {
                        // this is what we want!
                    }
                    res => panic!("unexpected result: {:?}", res),
                }
            })
            .expect("can spawn a thread");

        let terminator = in_guest.wait_and_then(move || {
            thread::spawn(move || {
                assert_eq!(kill_switch.terminate(), Ok(KillSuccess::Signalled));
            })
        });

        t.join().unwrap();
        terminator.join().unwrap();
    })
}

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

#[test]
fn terminate_in_hostcall() {
    test_instance_with_instrumented_guest_entry(|mut inst| {
        let kill_switch = inst.kill_switch();
        let in_hostcall = inst.lock_testpoints.instance_lock_before_exiting_hostcall.wait_at();

        let guest = thread::Builder::new()
            .name("guest".to_owned())
            .spawn(move || {
                match inst.run("run_slow_hostcall", &[]) {
                    Err(Error::RuntimeTerminated(TerminationDetails::Remote)) => {},
                    res => panic!("unexpectd result: {:?}", res),
                }
            })
            .expect("can spawn thread to run guest");

        in_hostcall.wait_and_then(|| {
            kill_switch.terminate().expect("can terminate in hostcall");
        });

        guest.join().expect("guest exits without panic");
    })
}

// Terminating an instance twice works, does not explode, and the second termination is an `Err`
// because the instance is no longer terminable.
#[test]
fn double_terminate() {
    test_instance_with_instrumented_guest_entry(|mut inst| {
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

        // At this point the first `KillSwitch` has completed terminating the instance. Now try
        // again and make sure there's no boom.
        assert_eq!(
            second_kill_switch.terminate(),
            Err(KillError::NotTerminable)
        );

        // Allow the instance to reset and run a new function after termination.
        guest_exit_testpoint.wait();

        // And after the instance successfully runs a test function, it exits without error.
        guest.join().expect("guest stops running");
    })
}
