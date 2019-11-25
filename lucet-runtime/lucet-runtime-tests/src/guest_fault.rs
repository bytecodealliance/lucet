use crate::helpers::{MockExportBuilder, MockModuleBuilder};
use lucet_module::{FunctionPointer, TrapCode, TrapSite};
use lucet_runtime_internals::module::Module;
use lucet_runtime_internals::vmctx::lucet_vmctx;
use std::sync::Arc;

pub fn mock_traps_module() -> Arc<dyn Module> {
    extern "C" fn onetwothree(_vmctx: *mut lucet_vmctx) -> std::os::raw::c_int {
        123
    }

    extern "C" fn hostcall_main(vmctx: *mut lucet_vmctx) -> () {
        extern "C" {
            // actually is defined in this file
            fn hostcall_test(vmctx: *mut lucet_vmctx);
        }
        unsafe {
            hostcall_test(vmctx);
            std::hint::unreachable_unchecked();
        }
    }

    extern "C" fn infinite_loop(_vmctx: *mut lucet_vmctx) -> () {
        loop {}
    }

    extern "C" fn fatal(vmctx: *mut lucet_vmctx) -> () {
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
            *heap_base.offset(0x200026000 * 16) = 0;
        }
    }

    extern "C" fn recoverable_fatal(_vmctx: *mut lucet_vmctx) -> () {
        use std::os::raw::c_char;
        extern "C" {
            fn guest_recoverable_get_ptr() -> *mut c_char;
        }

        unsafe {
            *guest_recoverable_get_ptr() = '\0' as c_char;
        }
    }

    // defined in `guest_fault/traps.S`
    extern "C" {
        fn guest_func_illegal_instr(vmctx: *mut lucet_vmctx);
        fn guest_func_oob(vmctx: *mut lucet_vmctx);
    }

    // Note: manually creating a trap manifest structure like this is almost certain to fragile at
    // best and flaky at worst. The test functions are provided in assembly in order to make it
    // marginally easier to keep things stable, but the magic numbers below may need to be updated
    // depending on the machine code that's generated.
    //
    // The easiest way I've found to update these is to use `layout asm` when running the tests in
    // gdb, and use the offsets it prints when it catches the signal. For example:
    //
    // >│0x5555556f53bd <guest_func_oob+29> movb   $0x0,0x10001(%rax) │
    //  │0x5555556f53c4 <guest_func_oob+36> add    $0x10,%rsp         │
    //  │0x5555556f53c8 <guest_func_oob+40> pop    %rbp               │
    //  │0x5555556f53c9 <guest_func_oob+41> retq                      |
    //
    // The offset below then should be 29, and the function length is 41.

    static ILLEGAL_INSTR_TRAPS: &'static [TrapSite] = &[TrapSite {
        offset: 8,
        code: TrapCode::BadSignature,
    }];

    static OOB_TRAPS: &'static [TrapSite] = &[TrapSite {
        offset: 29,
        code: TrapCode::HeapOutOfBounds,
    }];

    MockModuleBuilder::new()
        .with_export_func(MockExportBuilder::new(
            "onetwothree",
            FunctionPointer::from_usize(onetwothree as usize),
        ))
        .with_export_func(
            MockExportBuilder::new(
                "illegal_instr",
                FunctionPointer::from_usize(guest_func_illegal_instr as usize),
            )
            .with_func_len(11)
            .with_traps(ILLEGAL_INSTR_TRAPS),
        )
        .with_export_func(
            MockExportBuilder::new("oob", FunctionPointer::from_usize(guest_func_oob as usize))
                .with_func_len(41)
                .with_traps(OOB_TRAPS),
        )
        .with_export_func(MockExportBuilder::new(
            "hostcall_main",
            FunctionPointer::from_usize(hostcall_main as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "infinite_loop",
            FunctionPointer::from_usize(infinite_loop as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "fatal",
            FunctionPointer::from_usize(fatal as usize),
        ))
        .with_export_func(MockExportBuilder::new(
            "recoverable_fatal",
            FunctionPointer::from_usize(recoverable_fatal as usize),
        ))
        .build()
}

#[macro_export]
macro_rules! guest_fault_tests {
    ( $TestRegion:path ) => {
        use lazy_static::lazy_static;
        use libc::{c_void, pthread_kill, pthread_self, siginfo_t, SIGALRM, SIGSEGV};
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, lucet_hostcall_terminate, DlModule, Error, FaultDetails, Instance,
            Limits, Region, SignalBehavior, TerminationDetails, TrapCode,
        };
        use nix::sys::mman::{mmap, MapFlags, ProtFlags};
        use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{fork, ForkResult};
        use std::ptr;
        use std::sync::{Arc, Mutex};
        use $TestRegion as TestRegion;
        use $crate::guest_fault::mock_traps_module;
        use $crate::helpers::{
            test_ex, test_nonex, FunctionPointer, MockExportBuilder, MockModuleBuilder,
        };

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
                MapFlags::MAP_ANON | MapFlags::MAP_PRIVATE,
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

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_test(_vmctx: &mut Vmctx) {
            lucet_hostcall_terminate!(HOSTCALL_TEST_ERROR);
        }

        fn run_onetwothree(inst: &mut Instance) {
            let retval = inst
                .run("onetwothree", &[])
                .expect("instance runs")
                .unwrap_returned();
            assert_eq!(libc::c_int::from(retval), 123);
        }

        #[test]
        fn illegal_instr() {
            test_nonex(|| {
                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match inst.run("illegal_instr", &[]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode, Some(TrapCode::BadSignature));
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
                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match inst.run("oob", &[]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
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
                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match inst.run("hostcall_main", &[]) {
                    Err(Error::RuntimeTerminated(term)) => {
                        assert_eq!(
                            *term
                                .provided_details()
                                .expect("user terminated in hostcall")
                                .downcast_ref::<&'static str>()
                                .expect("error was str"),
                            HOSTCALL_TEST_ERROR,
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
                _trapcode: &Option<TrapCode>,
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
                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                // Install a signal handler that will override the fatal error and tell the sandbox to
                // continue executing. Obviously this is dangerous, but for this test it should be harmless.
                inst.set_signal_handler(signal_handler_continue);

                // set `recoverable_ptr` to point to a page that is not read/writable
                unsafe { recoverable_ptr_setup() };

                // Child code will call `guest_recoverable_get_ptr` and write to the pointer it
                // returns. This will initially cause a segfault. The signal handler will recover
                // from the segfault, map the page to read/write, and then return to the child
                // code. The child code will then succeed, and the instance will exit successfully.
                inst.run("recoverable_fatal", &[]).expect("instance runs");

                unsafe { recoverable_ptr_teardown() };
                drop(lock);
            });
        }

        #[test]
        fn fatal_terminate_signal_handler() {
            fn signal_handler_terminate(
                _inst: &Instance,
                _trapcode: &Option<TrapCode>,
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
                        let module = mock_traps_module();
                        let region = TestRegion::create(1, &Limits::default())
                            .expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");

                        // Install a signal handler that will override the fatal error and tell the sandbox to
                        // exit, but with a nonfatal error (should be an unknown fault)
                        inst.set_signal_handler(signal_handler_terminate);

                        // set `recoverable_ptr` to point to a page that is not read/writable
                        unsafe { recoverable_ptr_setup() };

                        // Child code will call `guest_recoverable_get_ptr` and write to the pointer it
                        // returns. This will initially cause a segfault. The signal handler will recover
                        // from the segfault, map the page to read/write, and then return to the child
                        // code. The child code will then succeed, and the instance will exit successfully.
                        match inst.run("recoverable_fatal", &[]) {
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
                let module = mock_traps_module();
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

                match inst.run("illegal_instr", &[]) {
                    Err(Error::RuntimeFault(details)) => {
                        assert_eq!(details.trapcode, Some(TrapCode::BadSignature));
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

            #[lucet_hostcall]
            pub fn sleepy_guest(_vmctx: &mut Vmctx) {
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
                        .with_export_func(MockExportBuilder::new(
                            "sleepy_guest",
                            FunctionPointer::from_usize(sleepy_guest as usize),
                        ))
                        .build();
                    let region =
                        TestRegion::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.run("sleepy_guest", &[]).expect("instance runs");
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
                            let module = mock_traps_module();
                            let region = TestRegion::create(1, &Limits::default())
                                .expect("region can be created");
                            let mut inst = region
                                .new_instance(module)
                                .expect("instance can be created");

                            inst.run("infinite_loop", &[]).expect("instance runs");
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
                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match fork().expect("can fork") {
                    ForkResult::Child => {
                        // Child code should run code that will make an OOB beyond the guard page. This will
                        // cause the entire process to abort before returning from `run`
                        inst.set_fatal_handler(handler);
                        inst.run("fatal", &[]).expect("instance runs");
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
                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");

                match fork().expect("can fork") {
                    ForkResult::Child => {
                        // Child code should run code that will make an OOB beyond the guard page. This will
                        // cause the entire process to abort before returning from `run`
                        inst.set_fatal_handler(fatal_handler_exit);
                        inst.run("fatal", &[]).expect("instance runs");
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

        #[test]
        fn sigaltstack_restores() {
            use libc::*;
            use std::mem::MaybeUninit;

            test_nonex(|| {
                // any alternate stack present before a thread runs an instance should be restored
                // after the instance returns
                let mut beforestack = MaybeUninit::<stack_t>::uninit();
                let beforestack = unsafe {
                    sigaltstack(std::ptr::null(), beforestack.as_mut_ptr());
                    beforestack.assume_init()
                };

                let module = mock_traps_module();
                let region =
                    TestRegion::create(1, &Limits::default()).expect("region can be created");
                let mut inst = region
                    .new_instance(module)
                    .expect("instance can be created");
                run_onetwothree(&mut inst);

                let mut afterstack = MaybeUninit::<stack_t>::uninit();
                let afterstack = unsafe {
                    sigaltstack(std::ptr::null(), afterstack.as_mut_ptr());
                    afterstack.assume_init()
                };

                assert_eq!(beforestack.ss_sp, afterstack.ss_sp);
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
