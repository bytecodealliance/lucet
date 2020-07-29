#[macro_export]
macro_rules! guest_fault_common_defs {
    () => {
        use common::{
            mock_traps_module, stack_testcase, wat_traps_module, HOSTCALL_TEST_ERROR,
            RECOVERABLE_PTR,
        };
        pub mod common {
            use lucet_module::{FunctionPointer, TrapCode, TrapSite};
            use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
            use lucet_runtime::{lucet_hostcall, lucet_hostcall_terminate, DlModule, Module};
            use std::sync::Arc;
            use tempfile::TempDir;
            use $crate::build::test_module_wasm;
            use $crate::helpers::{MockExportBuilder, MockModuleBuilder};

            pub const HOSTCALL_TEST_ERROR: &'static str = "hostcall_test threw an error!";

            #[lucet_hostcall]
            #[no_mangle]
            pub fn hostcall_test(_vmctx: &Vmctx) {
                lucet_hostcall_terminate!(HOSTCALL_TEST_ERROR);
            }

            #[lucet_hostcall]
            #[no_mangle]
            pub fn onetwothree(_vmctx: &Vmctx) -> i64 {
                123
            }

            pub struct OtherPanicPayload;

            #[lucet_hostcall]
            #[no_mangle]
            pub fn raise_other_panic(_vmctx: &Vmctx) {
                panic!(OtherPanicPayload);
            }

            pub static mut RECOVERABLE_PTR: *mut libc::c_char = std::ptr::null_mut();

            #[no_mangle]
            pub unsafe extern "C" fn guest_recoverable_get_ptr() -> *const libc::c_char {
                RECOVERABLE_PTR
            }

            pub fn wat_traps_module() -> Arc<dyn Module> {
                test_module_wasm("guest_fault", "guest.wat").expect("build and load module")
            }

            pub fn mock_traps_module() -> Arc<dyn Module> {
                extern "C" fn onetwothree(_vmctx: *const lucet_vmctx) -> std::os::raw::c_int {
                    123
                }

                extern "C" fn hostcall_main(vmctx: *const lucet_vmctx) {
                    extern "C" {
                        // actually is defined in this file
                        fn hostcall_test(vmctx: *const lucet_vmctx);
                    }
                    unsafe {
                        hostcall_test(vmctx);
                        std::hint::unreachable_unchecked();
                    }
                }

                extern "C" fn raise_other_panic_main(vmctx: *const lucet_vmctx) {
                    extern "C" {
                        // actually is defined in this file
                        fn raise_other_panic(vmctx: *const lucet_vmctx);
                    }
                    unsafe {
                        raise_other_panic(vmctx);
                        std::hint::unreachable_unchecked();
                    }
                }

                extern "C" fn infinite_loop(_vmctx: *const lucet_vmctx) {
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

                extern "C" fn recoverable_fatal(_vmctx: *const lucet_vmctx) {
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
                    fn guest_func_illegal_instr(vmctx: *const lucet_vmctx);
                    fn guest_func_oob(vmctx: *const lucet_vmctx);
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

                static ILLEGAL_INSTR_TRAPS: &[TrapSite] = &[TrapSite {
                    offset: 8,
                    code: TrapCode::BadSignature,
                }];

                static OOB_TRAPS: &[TrapSite] = &[TrapSite {
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
                        MockExportBuilder::new(
                            "oob",
                            FunctionPointer::from_usize(guest_func_oob as usize),
                        )
                        .with_func_len(41)
                        .with_traps(OOB_TRAPS),
                    )
                    .with_export_func(MockExportBuilder::new(
                        "hostcall_main",
                        FunctionPointer::from_usize(hostcall_main as usize),
                    ))
                    .with_export_func(MockExportBuilder::new(
                        "raise_other_panic_main",
                        FunctionPointer::from_usize(raise_other_panic_main as usize),
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
                        "hit_sigstack_guard_page",
                        FunctionPointer::from_usize(hit_sigstack_guard_page as usize),
                    ))
                    .with_export_func(MockExportBuilder::new(
                        "recoverable_fatal",
                        FunctionPointer::from_usize(recoverable_fatal as usize),
                    ))
                    .build()
            }

            pub fn stack_testcase(num_locals: usize) -> Result<Arc<DlModule>, anyhow::Error> {
                use lucetc::{Bindings, Lucetc, LucetcOpts};
                let native_build = Lucetc::try_from_bytes($crate::stack::generate_test_wat(
                    num_locals,
                    &["onetwothree"],
                    Some("      (i32.add (i32.wrap_i64 (call $onetwothree)))\n"),
                ))?
                .with_bindings(Bindings::from_str(
                    r#"
                            {
                                "env": {
                                    "onetwothree": "onetwothree"
                                }
                            }"#,
                )?);

                let workdir = TempDir::new().expect("create working directory");

                let so_file = workdir.path().join("out.so");

                native_build.shared_object_file(so_file.clone())?;

                let dlmodule = DlModule::load(so_file)?;

                Ok(dlmodule)
            }
        }

        #[test]
        fn ensure_linked() {
            lucet_runtime::lucet_internal_ensure_linked();
        }
    };
}

#[macro_export]
macro_rules! guest_fault_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {
        use lazy_static::lazy_static;
        use libc::{c_void, siginfo_t, SIGALRM, SIGBUS, SIGSEGV};
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, lucet_hostcall_terminate, DlModule, FaultDetails, Instance,
            Limits, Region, SignalBehavior, TerminationDetails, TrapCode,
        };
        use nix::sys::mman::{mmap, MapFlags, ProtFlags};
        use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{fork, ForkResult};
        use std::ptr;
        use std::sync::{Arc, Mutex};
        use $crate::helpers::{
            test_ex, test_nonex, with_unchanged_signal_handlers, FunctionPointer,
            MockExportBuilder, MockModuleBuilder,
        };

        lazy_static! {
            static ref RECOVERABLE_PTR_LOCK: Mutex<()> = Mutex::new(());
        }

        #[cfg(target_os = "linux")]
        const INVALID_PERMISSION_FAULT: libc::c_int = SIGSEGV;
        #[cfg(not(target_os = "linux"))]
        const INVALID_PERMISSION_FAULT: libc::c_int = SIGBUS;

        #[cfg(target_os = "linux")]
        const INVALID_PERMISSION_SIGNAL: Signal = Signal::SIGSEGV;
        #[cfg(not(target_os = "linux"))]
        const INVALID_PERMISSION_SIGNAL: Signal = Signal::SIGBUS;

        $(
            mod $region_id {
                use lazy_static::lazy_static;
                use libc::{c_void, pthread_kill, pthread_self, siginfo_t, SIGALRM, SIGBUS, SIGSEGV};
                use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
                use lucet_runtime::{
                    lucet_hostcall, lucet_hostcall_terminate, lucet_internal_ensure_linked, DlModule,
                    Error, FaultDetails, Instance, Limits, Region, RegionCreate, SignalBehavior, TerminationDetails,
                    TrapCode, UntypedRetVal,
                };
                use nix::sys::mman::{mmap, MapFlags, ProtFlags};
                use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
                use nix::sys::wait::{waitpid, WaitStatus};
                use nix::unistd::{fork, ForkResult};
                use std::ptr;
                use std::sync::{Arc, Mutex};
                use $TestRegion as TestRegion;
                use $crate::helpers::{
                    test_ex, test_nonex, with_unchanged_signal_handlers, FunctionPointer, MockExportBuilder, MockModuleBuilder,
                };
                use super::mock_traps_module;
                use super::stack_testcase;
                use super::wat_traps_module;

                unsafe fn recoverable_ptr_setup() {
                    assert!(super::RECOVERABLE_PTR.is_null());
                    super::RECOVERABLE_PTR = mmap(
                        ptr::null_mut(),
                        4096,
                        ProtFlags::PROT_NONE,
                        MapFlags::MAP_ANON | MapFlags::MAP_PRIVATE,
                        0,
                        0,
                    )
                        .expect("mmap succeeds") as *mut libc::c_char;
                    assert!(!super::RECOVERABLE_PTR.is_null());
                }

                unsafe fn recoverable_ptr_make_accessible() {
                    use nix::sys::mman::ProtFlags;

                    mprotect(
                        super::RECOVERABLE_PTR as *mut c_void,
                        4096,
                        ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                    )
                        .expect("mprotect succeeds");
                }

                unsafe fn recoverable_ptr_teardown() {
                    nix::sys::mman::munmap(super::RECOVERABLE_PTR as *mut c_void, 4096).expect("munmap succeeds");
                    super::RECOVERABLE_PTR = ptr::null_mut();
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
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                /// Test that the Lucet signal handler runs correctly when installed manually.
                fn illegal_instr_manual_signal() {
                    test_ex(|| {
                        with_unchanged_signal_handlers(|| {
                            let module = mock_traps_module();
                            let region =
                                <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                            let mut inst = region
                                .new_instance(module)
                                .expect("instance can be created");
                            inst.ensure_signal_handler_installed(false);

                            lucet_runtime::install_lucet_signal_handler();

                            match inst.run("illegal_instr", &[]) {
                                Err(Error::RuntimeFault(details)) => {
                                    assert_eq!(details.trapcode, Some(TrapCode::BadSignature));
                                }
                                res => panic!("unexpected result: {:?}", res),
                            }

                            // after a fault, can reset and run a normal function
                            inst.reset().expect("instance resets");

                            run_onetwothree(&mut inst);

                            lucet_runtime::remove_lucet_signal_handler();
                        });
                    })
                }

                #[test]
                /// Test that the Lucet signal handler runs correctly when installed manually, even when we
                /// don't keep a 1:1 ratio between install/remove.
                fn illegal_instr_manuals_signal() {
                    test_ex(|| {
                        with_unchanged_signal_handlers(|| {
                            let module = mock_traps_module();
                            let region =
                                <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                            let mut inst = region
                                .new_instance(module)
                                .expect("instance can be created");
                            inst.ensure_signal_handler_installed(false);

                            lucet_runtime::install_lucet_signal_handler();
                            // call it a few times; it shouldn't matter!
                            lucet_runtime::install_lucet_signal_handler();
                            lucet_runtime::install_lucet_signal_handler();

                            match inst.run("illegal_instr", &[]) {
                                Err(Error::RuntimeFault(details)) => {
                                    assert_eq!(details.trapcode, Some(TrapCode::BadSignature));
                                }
                                res => panic!("unexpected result: {:?}", res),
                            }

                            // after a fault, can reset and run a normal function
                            inst.reset().expect("instance resets");

                            run_onetwothree(&mut inst);

                            lucet_runtime::remove_lucet_signal_handler();
                            // call it a few times; it shouldn't matter!
                            lucet_runtime::remove_lucet_signal_handler();
                            lucet_runtime::remove_lucet_signal_handler();
                            lucet_runtime::remove_lucet_signal_handler();

                            // just reinstall once and make sure we catch the trap
                            lucet_runtime::install_lucet_signal_handler();

                            match inst.run("illegal_instr", &[]) {
                                Err(Error::RuntimeFault(details)) => {
                                    assert_eq!(details.trapcode, Some(TrapCode::BadSignature));
                                }
                                res => panic!("unexpected result: {:?}", res),
                            }

                            lucet_runtime::remove_lucet_signal_handler();
                        });
                    })
                }

                #[test]
                /// Test that the Lucet signal handler runs correctly when the sigstack is provided by the
                /// caller, rather than from the `Region`.
                ///
                /// The `signal_stack_size` of the `Region`'s limits is also set to zero, to show that we no
                /// longer validate the signal stack size on region creation.
                fn illegal_instr_manual_sigstack() {
                    use libc::*;
                    use std::mem::MaybeUninit;

                    test_nonex(|| {
                        let mut our_sigstack_alloc = vec![0; lucet_runtime::DEFAULT_SIGNAL_STACK_SIZE];
                        let our_sigstack = stack_t {
                            ss_sp: our_sigstack_alloc.as_mut_ptr() as *mut _,
                            ss_flags: 0,
                            ss_size: lucet_runtime::DEFAULT_SIGNAL_STACK_SIZE,
                        };
                        let mut beforestack = MaybeUninit::<stack_t>::uninit();
                        let beforestack = unsafe {
                            sigaltstack(&our_sigstack, beforestack.as_mut_ptr());
                            beforestack.assume_init()
                        };

                        let module = mock_traps_module();
                        let limits_no_sigstack = Limits::default()
                            .with_signal_stack_size(0);
                        let region =
                            <TestRegion as RegionCreate>::create(1, &limits_no_sigstack).expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");

                        inst.ensure_sigstack_installed(false);

                        match inst.run("illegal_instr", &[]) {
                            Err(Error::RuntimeFault(details)) => {
                                assert_eq!(details.trapcode, Some(TrapCode::BadSignature));
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        // after a fault, can reset and run a normal function
                        inst.reset().expect("instance resets");

                        run_onetwothree(&mut inst);

                        let mut afterstack = MaybeUninit::<stack_t>::uninit();
                        let afterstack = unsafe {
                            sigaltstack(&beforestack, afterstack.as_mut_ptr());
                            afterstack.assume_init()
                        };

                        assert_eq!(afterstack.ss_sp, our_sigstack_alloc.as_mut_ptr() as *mut _);
                    })
                }

                #[test]
                fn oob() {
                    test_nonex(|| {
                        let module = mock_traps_module();
                        let region =
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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

                // Ensure that guests can be successfully run after an instance faults, but without
                // resetting the guest.
                #[test]
                fn guest_after_fault_without_reset() {
                    test_nonex(|| {
                        let module = mock_traps_module();
                        let region =
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");

                        match inst.run("oob", &[]) {
                            Err(Error::RuntimeFault(details)) => {
                                assert_eq!(details.trapcode, Some(TrapCode::HeapOutOfBounds));
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        run_onetwothree(&mut inst);
                    });
                }

                #[test]
                fn hostcall_insufficient_stack() {
                    test_nonex(|| {
                        // NOTE: we must use `wat_traps_module` here because we want to test an
                        // artifact that only exists through `lucetc`: hostcall trampolines. The
                        // mock module will not have trampolines as lucetc generates, and this test
                        // will not see the expected failure if a mock module is used.
                        let module = wat_traps_module();

                        // Require that the entire stack be free when making a hostcall. Since at
                        // least some of the stack will always be used for the backstop, this has
                        // the effect of failing the check for any hostcall.
                        let impossible_hostcall_limits = Limits::default()
                            .with_hostcall_reservation(Limits::default().stack_size);
                        let region =
                            <TestRegion as RegionCreate>::create(
                                1,
                                &impossible_hostcall_limits
                            ).expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");

                        // Run the hostcall `onetwothree` because other than the hostcall stack
                        // limit check, the hostcall in question should complete successfully.
                        match inst.run("make_onetwothree_hostcall", &[]) {
                            Err(Error::RuntimeFault(details)) => {
                                assert_eq!(details.trapcode, Some(TrapCode::StackOverflow));
                            },
                            res => panic!("unexpected result: {:?}", res),
                        }

                        // after a fault, can reset and run a normal function
                        inst.reset().expect("instance resets");

                        run_onetwothree(&mut inst);
                    });
                }

                fn run(limits: &Limits, module: Arc<DlModule>, recursion_depth: i32) -> Result<UntypedRetVal, Error> {
                    let region = <TestRegion as RegionCreate>::create(1, limits).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.run("localpalooza", &[recursion_depth.into()])
                        .and_then(|rr| rr.returned())
                }

                fn expect_ok(limits: &Limits, module: Arc<DlModule>, recursion_depth: i32) {
                    assert!(run(limits, module, recursion_depth).is_ok());
                }

                fn expect_stack_overflow(limits: &Limits, module: Arc<DlModule>, recursion_depth: i32, probestack: bool) {
                    match run(limits, module, recursion_depth) {
                        Err(Error::RuntimeFault(details)) => {
                            // We should get a nonfatal trap due to the stack overflow.
                            assert_eq!(details.fatal, false);
                            assert_eq!(details.trapcode, Some(TrapCode::StackOverflow));
                            if probestack {
                                // Make sure we overflowed in the stack probe as expected
                                //
                                // TODO: this no longer differentiates between different stack overflow
                                // sites after moving the stack probe into lucetc; figure out a way to
                                // provide that information or just wait till we can do the stack probe as a
                                // global symbol again
                                let addr_details =
                                    details.rip_addr_details.expect("can look up addr details");
                                assert!(addr_details.in_module_code);
                            }
                        }
                        res => panic!("unexpected result: {:?}", res),
                    }
                }

                #[test]
                // Exhausting most stack space with the default hostcall reservation will fail,
                // we've used far more than the limit in the guest.
                fn expect_hostcall_reservation_stack_overflow_locals64_441() {
                    expect_stack_overflow(
                        &Limits::default(),
                        stack_testcase(64 - 4).expect("generate stack_testcase 64"),
                        461,
                        true,
                    );
                }

                #[test]
                // Exhausting most stack space with hostcall reservation set very low should work;
                // the hostcall `onetwothree` uses little stack space.
                fn expect_no_stack_overflow_locals64_441() {
                    expect_ok(
                        &Limits::default()
                            .with_hostcall_reservation(128),
                        stack_testcase(64 - 4).expect("generate stack_testcase 64"),
                        461,
                    );
                }

                #[test]
                fn hostcall_error() {
                    test_nonex(|| {
                        let module = mock_traps_module();
                        let region =
                            <TestRegion as RegionCreate>::create(1, &Limits::default())
                            .expect("region can be created");
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
                                    super::HOSTCALL_TEST_ERROR,
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
                fn raise_other_panic() {
                    test_nonex(|| {
                        let module = mock_traps_module();
                        let region =
                            <TestRegion as RegionCreate>::create(1, &Limits::default())
                            .expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");

                        match inst.run("raise_other_panic_main", &[]) {
                            Err(Error::RuntimeTerminated(TerminationDetails::OtherPanic(payload))) => {
                                assert!(payload.is::<crate::common::OtherPanicPayload>());
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        // after a fault, can reset and run a normal function; in practice we would
                        // want to reraise the panic most of the time, but this should still work
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
                        assert!(signum == super::INVALID_PERMISSION_FAULT);

                        // The fault was caused by writing to a protected page at `recoverable_ptr`.  Make that
                        // no longer be a fault
                        unsafe { recoverable_ptr_make_accessible() };

                        // Now the guest code can continue
                        SignalBehavior::Continue
                    }
                    test_nonex(|| {
                        // make sure only one test using super::RECOVERABLE_PTR is running at once
                        let lock = super::RECOVERABLE_PTR_LOCK.lock().unwrap();
                        let module = mock_traps_module();
                        let region =
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        assert!(signum == super::INVALID_PERMISSION_FAULT);

                        // Terminate guest
                        SignalBehavior::Terminate
                    }
                    test_ex(|| {
                        // // make sure only one test using super::RECOVERABLE_PTR is running at once
                        let lock = super::RECOVERABLE_PTR_LOCK.lock().unwrap();
                        match fork().expect("can fork") {
                            ForkResult::Child => {
                                let module = mock_traps_module();
                                let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
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
                        static ref HOST_FAULT_TRIGGERED: Mutex<bool> = Mutex::new(false);
                    }

                    extern "C" fn host_sigsegv_handler(
                        signum: libc::c_int,
                        _siginfo_ptr: *mut siginfo_t,
                        _ucontext_ptr: *mut c_void,
                    ) {
                        assert!(signum == super::INVALID_PERMISSION_FAULT);
                        unsafe { recoverable_ptr_make_accessible() };
                        *HOST_FAULT_TRIGGERED.lock().unwrap() = true;
                    }
                    test_ex(|| {
                        with_unchanged_signal_handlers(|| {
                            // make sure only one test using super::RECOVERABLE_PTR is running at once
                            let recoverable_ptr_lock = super::RECOVERABLE_PTR_LOCK.lock().unwrap();
                            let module = mock_traps_module();
                            let region =
                                <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                            let mut inst = region
                                .new_instance(module)
                                .expect("instance can be created");

                            let sa = SigAction::new(
                                SigHandler::SigAction(host_sigsegv_handler),
                                SaFlags::SA_RESTART,
                                SigSet::all(),
                            );
                            let before_sa = unsafe {
                                sigaction(super::INVALID_PERMISSION_SIGNAL, &sa).expect("sigaction succeeds")
                            };

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
                            *HOST_FAULT_TRIGGERED.lock().unwrap() = false;

                            // accessing this should trigger the segfault
                            unsafe {
                                *super::RECOVERABLE_PTR = 0;
                            }

                            assert!(*HOST_FAULT_TRIGGERED.lock().unwrap());

                            // clean up
                            unsafe {
                                recoverable_ptr_teardown();
                                sigaction(super::INVALID_PERMISSION_SIGNAL, &before_sa)
                                    .expect("sigaction succeeds");
                            }

                            drop(recoverable_ptr_lock);
                        })
                    })
                }

                #[test]
                fn sigsegv_handler_during_guest() {
                    lazy_static! {
                        static ref HOST_FAULT_TRIGGERED: Mutex<bool> = Mutex::new(false);
                    }

                    extern "C" fn host_sigsegv_handler(
                        signum: libc::c_int,
                        _siginfo_ptr: *mut siginfo_t,
                        _ucontext_ptr: *mut c_void,
                    ) {
                        assert!(signum == super::INVALID_PERMISSION_FAULT);
                        unsafe { recoverable_ptr_make_accessible() };
                        *HOST_FAULT_TRIGGERED.lock().unwrap() = true;
                    }

                    #[lucet_hostcall]
                    pub fn sleepy_guest(_vmctx: &Vmctx) {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }

                    test_ex(|| {
                        with_unchanged_signal_handlers(|| {
                            // make sure only one test using super::RECOVERABLE_PTR is running at once
                            let recoverable_ptr_lock = super::RECOVERABLE_PTR_LOCK.lock().unwrap();

                            let sa = SigAction::new(
                                SigHandler::SigAction(host_sigsegv_handler),
                                SaFlags::SA_RESTART,
                                SigSet::empty(),
                            );

                            let saved_fault_sa = unsafe {
                                sigaction(super::INVALID_PERMISSION_SIGNAL, &sa).expect("sigaction succeeds")
                            };

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
                                let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
                                    .expect("region can be created");
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
                            *HOST_FAULT_TRIGGERED.lock().unwrap() = false;

                            // accessing this should trigger the segfault
                            unsafe {
                                *super::RECOVERABLE_PTR = 0;
                            }

                            assert!(*HOST_FAULT_TRIGGERED.lock().unwrap());

                            child.join().expect("can join on child");

                            // clean up
                            unsafe {
                                recoverable_ptr_teardown();
                                // sigaltstack(&saved_sigstack).expect("sigaltstack succeeds");
                                sigaction(super::INVALID_PERMISSION_SIGNAL, &saved_fault_sa)
                                    .expect("sigaction succeeds");
                            }

                            drop(recoverable_ptr_lock);
                        })
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
                                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default())
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
                                    *super::RECOVERABLE_PTR = 0;
                                }
                            }
                            ForkResult::Parent { child } => {
                                match waitpid(Some(child), None).expect("can wait on child") {
                                    WaitStatus::Signaled(_, sig, _) => {
                                        assert_eq!(sig, super::INVALID_PERMISSION_SIGNAL);
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
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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

                #[test]
                fn hit_sigstack_guard_page() {
                    fn handler(_inst: &Instance) -> ! {
                        std::process::abort()
                    }
                    test_ex(|| {
                        let module = mock_traps_module();
                        match fork().expect("can fork") {
                            ForkResult::Child => {
                                let region =
                                    <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                                let mut inst = region
                                    .new_instance(module)
                                    .expect("instance can be created");

                                // Child code should run code that will hit the signal stack's guard
                                // page. This will cause the entire process to abort before returning from
                                // `run`
                                inst.set_fatal_handler(handler);
                                inst.run("hit_sigstack_guard_page", &[])
                                    .expect("instance runs");
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
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                            <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
            }
        )*
    };
}
