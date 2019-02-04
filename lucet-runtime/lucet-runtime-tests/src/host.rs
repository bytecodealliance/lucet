#[macro_export]
macro_rules! host_tests {
    ( $TestRegion:path ) => {
        use libc::c_void;
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{DlModule, Error, Limits, Region, TrapCodeType};
        use $TestRegion as TestRegion;
        use $crate::helpers::DlModuleExt;

        const NULL_MOD_PATH: &'static str = "tests/build/host_guests/null.so";
        const HELLO_MOD_PATH: &'static str = "tests/build/host_guests/hello.so";
        const HOSTCALL_ERROR_MOD_PATH: &'static str = "tests/build/host_guests/hostcall_error.so";
        const FPE_MOD_PATH: &'static str = "tests/build/host_guests/fpe.so";

        #[test]
        fn load_module() {
            let _module = DlModule::load_test(NULL_MOD_PATH).expect("module loads");
        }

        #[test]
        fn load_nonexistent_module() {
            let module = DlModule::load_test("nonexistent_sandbox");
            assert!(module.is_err());
        }

        #[no_mangle]
        extern "C" fn hostcall_test_func_hello(
            vmctx: *mut lucet_vmctx,
            hello_ptr: u32,
            hello_len: u32,
        ) {
            unsafe {
                let mut vmctx = Vmctx::from_raw(vmctx);
                let confirmed_hello = vmctx.embed_ctx() as *mut bool;
                let heap = vmctx.heap();
                let hello = heap.as_ptr() as usize + hello_ptr as usize;
                if !vmctx.check_heap(hello as *const c_void, hello_len as usize) {
                    vmctx.terminate(std::ptr::null_mut());
                }
                let hello = std::slice::from_raw_parts(hello as *const u8, hello_len as usize);
                if hello.starts_with(b"hello") {
                    *confirmed_hello = true;
                }
            }
        }

        const ERROR_MESSAGE: &'static str = "hostcall_test_func_hostcall_error";
        #[no_mangle]
        extern "C" fn hostcall_test_func_hostcall_error(vmctx: *mut lucet_vmctx) {
            let info = Box::new(ERROR_MESSAGE);
            unsafe { Vmctx::from_raw(vmctx).terminate(Box::into_raw(info) as *mut c_void) }
        }

        #[test]
        fn instantiate_null() {
            let module = DlModule::load_test(NULL_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");
        }

        #[test]
        fn run_null() {
            let module = DlModule::load_test(NULL_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");
            inst.run(b"main", &[]).expect("instance runs");
        }

        #[test]
        fn run_hello() {
            let module = DlModule::load_test(HELLO_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut confirm_hello = false;

            let mut inst = region
                .new_instance_with_ctx(
                    Box::new(module),
                    (&mut confirm_hello) as *mut bool as *mut c_void,
                )
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            assert!(confirm_hello);
        }

        #[test]
        fn run_hostcall_error() {
            let module = DlModule::load_test(HOSTCALL_ERROR_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            match inst.run(b"main", &[]) {
                Err(Error::RuntimeTerminated(details)) => {
                    let info = unsafe { Box::from_raw(details.info as *mut &'static str) };
                    assert_eq!(*info, ERROR_MESSAGE);
                }
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn run_fpe() {
            let module = DlModule::load_test(FPE_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            match inst.run(b"trigger_div_error", &[0u64.into()]) {
                Err(Error::RuntimeFault(details)) => {
                    assert_eq!(details.trapcode.ty, TrapCodeType::IntegerDivByZero);
                }
                res => {
                    panic!("unexpected result: {:?}", res);
                }
            }
        }
    };
}
