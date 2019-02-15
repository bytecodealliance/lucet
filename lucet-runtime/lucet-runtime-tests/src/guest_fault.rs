#[macro_export]
macro_rules! guest_fault_tests {
    ( $TestRegion:path ) => {
        use lazy_static::lazy_static;
        use libc::{c_void, siginfo_t, SIGSEGV};
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            DlModule, Error, FaultDetails, Instance, Limits, Region, SignalBehavior, TrapCode,
            TrapCodeType,
        };
        use nix::sys::mman::{mmap, MapFlags, ProtFlags};
        use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{fork, ForkResult};
        use std::ptr;
        use std::sync::{Arc, Mutex};
        use $TestRegion as TestRegion;
        use $crate::helpers::{
            guest_module_path, test_ex, test_nonex, DlModuleExt, MockModuleBuilder,
        };

        const TRAPS_SANDBOX_PATH: &'static str = "lucet-runtime-c/test/build/guest_faults/traps.so";
        const HOSTCALL_ERROR_SANDBOX_PATH: &'static str =
            "lucet-runtime-c/test/build/guest_faults/hostcall_error.so";

        lazy_static! {
            static ref RECOVERABLE_PTR_LOCK: Mutex<()> = Mutex::new(());
        }

        static mut RECOVERABLE_PTR: *mut libc::c_char = ptr::null_mut();

        unsafe fn recoverable_ptr_setup() {
            assert!(RECOVERABLE_PTR.is_null());
            RECOVERABLE_PTR = mmap(
                ptr::null_mut(),
                4096,
                ProtFlags::PROT_NONE,
                MapFlags::MAP_ANONYMOUS | MapFlags::MAP_PRIVATE,
                0,
                0,
            )
            .expect("mmap succeeds") as *mut libc::c_char;
            assert!(!RECOVERABLE_PTR.is_null());
        }

        unsafe fn recoverable_ptr_make_accessible() {
            use nix::sys::mman::ProtFlags;

            mprotect(
                RECOVERABLE_PTR as *mut c_void,
                4096,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            )
            .expect("mprotect succeeds");
        }

        unsafe fn recoverable_ptr_teardown() {
            nix::sys::mman::munmap(RECOVERABLE_PTR as *mut c_void, 4096).expect("munmap succeeds");
            RECOVERABLE_PTR = ptr::null_mut();
        }

        #[no_mangle]
        unsafe extern "C" fn guest_recoverable_get_ptr() -> *const libc::c_char {
            RECOVERABLE_PTR
        }

        static HOSTCALL_TEST_ERROR: &'static str = "hostcall_test threw an error!";

        #[no_mangle]
        unsafe extern "C" fn hostcall_test(vmctx: *mut lucet_vmctx) {
            Vmctx::from_raw(vmctx).terminate(HOSTCALL_TEST_ERROR.as_ptr() as *mut c_void);
        }

        fn run_onetwothree(inst: &mut Instance) {
            let retval = inst.run(b"onetwothree", &[]).expect("instance runs");
            assert_eq!(libc::c_int::from(retval), 123);
        }

        #[test]
        fn illegal_instr() {
            test_nonex(|| {
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match inst.run(b"illegal_instr", &[]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode.ty, TrapCodeType::BadSignature);
                        let addr_details = details.rip_addr_details.unwrap();
                        assert!(addr_details.in_module_code);
                        assert_eq!(
                            addr_details.file_name,
                            Some(
                                guest_module_path(TRAPS_SANDBOX_PATH)
                                    .to_string_lossy()
                                    .into_owned()
                            )
                        );
                        assert_eq!(
                            addr_details.sym_name,
                            Some(format!("guest_func_illegal_instr"))
                        );
                    }
                    res => panic!("unexpected result: {:?}", res),
                }

                // after a fault, can reset and run a normal function
                inst.reset().expect("instance resets");

                run_onetwothree(&mut inst);
            })
        }

        #[test]
        fn oob() {
            test_nonex(|| {
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match inst.run(b"oob", &[]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode.ty, TrapCodeType::HeapOutOfBounds);
                        let addr_details = details.rip_addr_details.unwrap();
                        assert!(addr_details.in_module_code);
                        assert_eq!(
                            addr_details.file_name,
                            Some(
                                guest_module_path(TRAPS_SANDBOX_PATH)
                                    .to_string_lossy()
                                    .into_owned()
                            )
                        );
                        assert_eq!(addr_details.sym_name, Some(format!("guest_func_oob")));
                    }
                    res => panic!("unexpected result: {:?}", res),
                }

                // after a fault, can reset and run a normal function
                inst.reset().expect("instance resets");

                run_onetwothree(&mut inst);
            });
        }

        #[test]
        fn hostcall_error() {
            test_nonex(|| {
                let module =
                    DlModule::load_test(HOSTCALL_ERROR_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match inst.run(b"main", &[]) {
                    Err(Error::RuntimeTerminated(details)) => {
                        assert_eq!(
                            details.unwrap().info,
                            HOSTCALL_TEST_ERROR.as_ptr() as *mut c_void
                        );
                    }
                    res => panic!("unexpected result: {:?}", res),
                }

                // after a fault, can reset and run a normal function
                inst.reset().expect("instance resets");

                run_onetwothree(&mut inst);
            });
        }

        #[test]
        fn fatal_continue_signal_handler() {
            fn signal_handler_continue(
                _inst: &Instance,
                _trapcode: &TrapCode,
                signum: libc::c_int,
                _siginfo_ptr: *const siginfo_t,
                _ucontext_ptr: *const c_void,
            ) -> SignalBehavior {
                // Triggered by a SIGSEGV writing to protected page
                assert!(signum == SIGSEGV);

                // The fault was caused by writing to a protected page at `recoverable_ptr`.  Make that
                // no longer be a fault
                unsafe { recoverable_ptr_make_accessible() };

                // Now the guest code can continue
                SignalBehavior::Continue
            }
            test_nonex(|| {
                // make sure only one test using RECOVERABLE_PTR is running at once
                let lock = RECOVERABLE_PTR_LOCK.lock().unwrap();
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                // Install a signal handler that will override the fatal error and tell the sandbox to
                // continue executing. Obviously this is dangerous, but for this test it should be harmless.
                inst.signal_handler = signal_handler_continue;

                // set `recoverable_ptr` to point to a page that is not read/writable
                unsafe { recoverable_ptr_setup() };

                // Child code will call `guest_recoverable_get_ptr` and write to the pointer it
                // returns. This will initially cause a segfault. The signal handler will recover
                // from the segfault, map the page to read/write, and then return to the child
                // code. The child code will then succeed, and the instance will exit successfully.
                inst.run(b"recoverable_fatal", &[]).expect("instance runs");

                unsafe { recoverable_ptr_teardown() };
                drop(lock);
            });
        }

        #[test]
        fn fatal_terminate_signal_handler() {
            fn signal_handler_terminate(
                _inst: &Instance,
                _trapcode: &TrapCode,
                signum: libc::c_int,
                _siginfo_ptr: *const siginfo_t,
                _ucontext_ptr: *const c_void,
            ) -> SignalBehavior {
                // Triggered by a SIGSEGV writing to protected page
                assert!(signum == SIGSEGV);

                // Terminate guest
                SignalBehavior::Terminate
            }
            test_ex(|| {
                // // make sure only one test using RECOVERABLE_PTR is running at once
                let lock = RECOVERABLE_PTR_LOCK.lock().unwrap();
                match fork().expect("can fork") {
                    ForkResult::Child => {
                        let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                        let region = TestRegion::create(1, &Limits::default())
                            .expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");

                        // Install a signal handler that will override the fatal error and tell the sandbox to
                        // exit, but with a nonfatal error (should be an unknown fault)
                        inst.signal_handler = signal_handler_terminate;

                        // set `recoverable_ptr` to point to a page that is not read/writable
                        unsafe { recoverable_ptr_setup() };

                        // Child code will call `guest_recoverable_get_ptr` and write to the pointer it
                        // returns. This will initially cause a segfault. The signal handler will recover
                        // from the segfault, map the page to read/write, and then return to the child
                        // code. The child code will then succeed, and the instance will exit successfully.
                        match inst.run(b"recoverable_fatal", &[]) {
                            Err(Error::RuntimeTerminated(_)) => (),
                            res => panic!("unexpected result: {:?}", res),
                        }

                        unsafe { recoverable_ptr_teardown() };
                        // don't want this child continuing to test harness code
                        std::process::exit(0);
                    }
                    ForkResult::Parent { child } => {
                        match waitpid(Some(child), None).expect("can wait on child") {
                            WaitStatus::Exited(_, code) => {
                                assert_eq!(code, 0);
                            }
                            ws => panic!("unexpected wait status: {:?}", ws),
                        }
                    }
                }
                drop(lock);
            })
        }

        #[test]
        fn sigsegv_handler_saved_restored() {
            lazy_static! {
                static ref HOST_SIGSEGV_TRIGGERED: Mutex<bool> = Mutex::new(false);
            }

            extern "C" fn host_sigsegv_handler(
                signum: libc::c_int,
                _siginfo_ptr: *mut siginfo_t,
                _ucontext_ptr: *mut c_void,
            ) {
                // Triggered by a SIGSEGV writing to protected page
                assert!(signum == SIGSEGV);
                unsafe { recoverable_ptr_make_accessible() };
                *HOST_SIGSEGV_TRIGGERED.lock().unwrap() = true;
            }
            test_ex(|| {
                // make sure only one test using RECOVERABLE_PTR is running at once
                let recoverable_ptr_lock = RECOVERABLE_PTR_LOCK.lock().unwrap();
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                let sa = SigAction::new(
                    SigHandler::SigAction(host_sigsegv_handler),
                    SaFlags::SA_RESTART,
                    SigSet::all(),
                );
                unsafe { sigaction(Signal::SIGSEGV, &sa).expect("sigaction succeeds") };

                match inst.run(b"illegal_instr", &[]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode.ty, TrapCodeType::BadSignature);
                        let addr_details = details.rip_addr_details.unwrap();
                        assert!(addr_details.in_module_code);
                        assert_eq!(
                            addr_details.file_name,
                            Some(
                                guest_module_path(TRAPS_SANDBOX_PATH)
                                    .to_string_lossy()
                                    .into_owned()
                            )
                        );
                        assert_eq!(
                            addr_details.sym_name,
                            Some(format!("guest_func_illegal_instr"))
                        );
                    }
                    res => panic!("unexpected result: {:?}", res),
                }

                // now make sure that the host sigaction has been restored
                unsafe {
                    recoverable_ptr_setup();
                }
                *HOST_SIGSEGV_TRIGGERED.lock().unwrap() = false;

                // accessing this should trigger the segfault
                unsafe {
                    *RECOVERABLE_PTR = 0;
                }

                assert!(*HOST_SIGSEGV_TRIGGERED.lock().unwrap());

                // clean up
                unsafe {
                    recoverable_ptr_teardown();
                    sigaction(
                        Signal::SIGSEGV,
                        &SigAction::new(SigHandler::SigDfl, SaFlags::SA_RESTART, SigSet::empty()),
                    )
                    .expect("sigaction succeeds");
                }

                drop(recoverable_ptr_lock);
            })
        }

        #[test]
        fn alarm() {
            extern "C" fn timeout_handler(signum: libc::c_int) {
                assert!(signum == libc::SIGALRM);
                std::process::exit(3);
            }
            test_ex(|| {
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                inst.fatal_handler = fatal_handler_exit;

                match fork().expect("can fork") {
                    ForkResult::Child => {
                        // set up alarm handler and pend an alarm in 1 second
                        unsafe {
                            // child process doesn't have any contention for installed signal handlers, so
                            // we don't need to grab the lock exclusively here
                            sigaction(
                                Signal::SIGALRM,
                                &SigAction::new(
                                    SigHandler::Handler(timeout_handler),
                                    SaFlags::empty(),
                                    SigSet::empty(),
                                ),
                            )
                            .expect("sigaction succeeds");
                        }
                        nix::unistd::alarm::set(1);

                        // run guest code that loops forever
                        inst.run(b"infinite_loop", &[]).expect("instance runs");
                        // show that we never get here
                        std::process::exit(1);
                    }
                    ForkResult::Parent { child } => {
                        match waitpid(Some(child), None).expect("can wait on child") {
                            WaitStatus::Exited(_, code) => {
                                assert_eq!(code, 3);
                            }
                            ws => panic!("unexpected wait status: {:?}", ws),
                        }
                    }
                }
            })
        }

        #[test]
        fn sigsegv_handler_during_guest() {
            lazy_static! {
                static ref HOST_SIGSEGV_TRIGGERED: Mutex<bool> = Mutex::new(false);
            }

            extern "C" fn host_sigsegv_handler(
                signum: libc::c_int,
                _siginfo_ptr: *mut siginfo_t,
                _ucontext_ptr: *mut c_void,
            ) {
                // Triggered by a SIGSEGV writing to protected page
                assert!(signum == SIGSEGV);
                unsafe { recoverable_ptr_make_accessible() };
                *HOST_SIGSEGV_TRIGGERED.lock().unwrap() = true;
            }

            extern "C" fn sleepy_guest(_vmctx: *const lucet_vmctx) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }

            test_ex(|| {
                // make sure only one test using RECOVERABLE_PTR is running at once
                let recoverable_ptr_lock = RECOVERABLE_PTR_LOCK.lock().unwrap();

                let sa = SigAction::new(
                    SigHandler::SigAction(host_sigsegv_handler),
                    SaFlags::SA_RESTART,
                    SigSet::empty(),
                );

                let saved_sa =
                    unsafe { sigaction(Signal::SIGSEGV, &sa).expect("sigaction succeeds") };

                // The original thread will run `sleepy_guest`, and the new thread will dereference a null
                // pointer after a delay. This should lead to a sigsegv while the guest is running,
                // therefore testing that the host signal gets re-raised.
                let child = std::thread::spawn(|| {
                    let module = MockModuleBuilder::new()
                        .with_export_func(b"sleepy_guest", sleepy_guest as *const extern "C" fn())
                        .build();
                    let region =
                        TestRegion::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.run(b"sleepy_guest", &[]).expect("instance runs");
                });

                // now trigger a segfault in the middle of running the guest
                std::thread::sleep(std::time::Duration::from_millis(10));
                unsafe {
                    recoverable_ptr_setup();
                }
                *HOST_SIGSEGV_TRIGGERED.lock().unwrap() = false;

                // accessing this should trigger the segfault
                unsafe {
                    *RECOVERABLE_PTR = 0;
                }

                assert!(*HOST_SIGSEGV_TRIGGERED.lock().unwrap());

                child.join().expect("can join on child");

                // clean up
                unsafe {
                    recoverable_ptr_teardown();
                    // sigaltstack(&saved_sigstack).expect("sigaltstack succeeds");
                    sigaction(Signal::SIGSEGV, &saved_sa).expect("sigaction succeeds");
                }

                drop(recoverable_ptr_lock);
            })
        }

        #[test]
        fn handle_host_signal() {
            test_ex(|| {
                match fork().expect("can fork") {
                    ForkResult::Child => {
                        unsafe {
                            recoverable_ptr_setup();
                        }
                        // Child code will fork a new thread. The original thread will run `infinite_loop`,
                        // and the new thread will dereference a null pointer after 500ms. This should lead
                        // to a sigsegv while the guest is running, therefore testing that the host signal
                        // gets re-raised.
                        std::thread::spawn(|| {
                            let module =
                                DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                            let region = TestRegion::create(1, &Limits::default())
                                .expect("region can be created");
                            let mut inst = region
                                .new_instance(module)
                                .expect("instance can be created");

                            inst.run(b"infinite_loop", &[]).expect("instance runs");
                            unreachable!()
                        });

                        std::thread::sleep(std::time::Duration::from_millis(500));
                        // accessing this should trigger the segfault
                        unsafe {
                            *RECOVERABLE_PTR = 0;
                        }
                    }
                    ForkResult::Parent { child } => {
                        match waitpid(Some(child), None).expect("can wait on child") {
                            WaitStatus::Signaled(_, sig, _) => {
                                assert_eq!(sig, Signal::SIGSEGV);
                            }
                            ws => panic!("unexpected wait status: {:?}", ws),
                        }
                    }
                }
            })
        }

        #[test]
        fn fatal_abort() {
            fn handler(_inst: &Instance) -> ! {
                std::process::abort()
            }
            test_ex(|| {
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match fork().expect("can fork") {
                    ForkResult::Child => {
                        // Child code should run code that will make an OOB beyond the guard page. This will
                        // cause the entire process to abort before returning from `run`
                        inst.fatal_handler = handler;
                        inst.run(b"fatal", &[]).expect("instance runs");
                        // Show that we never get here:
                        std::process::exit(1);
                    }
                    ForkResult::Parent { child } => {
                        match waitpid(Some(child), None).expect("can wait on child") {
                            WaitStatus::Signaled(_, sig, _) => {
                                assert_eq!(sig, Signal::SIGABRT);
                            }
                            ws => panic!("unexpected wait status: {:?}", ws),
                        }
                    }
                }
            })
        }

        fn fatal_handler_exit(_inst: &Instance) -> ! {
            std::process::exit(42)
        }

        #[test]
        fn fatal_handler() {
            test_ex(|| {
                let module = DlModule::load_test(TRAPS_SANDBOX_PATH).expect("module loads");
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match fork().expect("can fork") {
                    ForkResult::Child => {
                        // Child code should run code that will make an OOB beyond the guard page. This will
                        // cause the entire process to abort before returning from `run`
                        inst.fatal_handler = fatal_handler_exit;
                        inst.run(b"fatal", &[]).expect("instance runs");
                        // Show that we never get here:
                        std::process::exit(1);
                    }
                    ForkResult::Parent { child } => {
                        match waitpid(Some(child), None).expect("can wait on child") {
                            WaitStatus::Exited(_, code) => {
                                assert_eq!(code, 42);
                            }
                            ws => panic!("unexpected wait status: {:?}", ws),
                        }
                    }
                }
            })
        }

        // TODO: remove this once `nix` PR https://github.com/nix-rust/nix/pull/991 is merged
        pub unsafe fn mprotect(
            addr: *mut c_void,
            length: libc::size_t,
            prot: ProtFlags,
        ) -> nix::Result<()> {
            nix::errno::Errno::result(libc::mprotect(addr, length, prot.bits())).map(drop)
        }
    };
}
