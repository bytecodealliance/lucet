#[macro_export]
macro_rules! host_tests {
    ( $TestRegion:path ) => {
        use lazy_static::lazy_static;
        use libc::c_void;
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, lucet_hostcall_terminate, DlModule, Error, Limits, Region,
            TerminationDetails, TrapCode,
        };
        use std::sync::{Arc, Mutex};
        use $TestRegion as TestRegion;
        use $crate::build::test_module_c;
        use $crate::helpers::{FunctionPointer, MockExportBuilder, MockModuleBuilder};
        #[test]
        fn load_module() {
            let _module = test_module_c("host", "trivial.c").expect("build and load module");
        }

        #[test]
        fn load_nonexistent_module() {
            let module = DlModule::load("/non/existient/file");
            assert!(module.is_err());
        }

        const ERROR_MESSAGE: &'static str = "hostcall_test_func_hostcall_error";

        lazy_static! {
            static ref HOSTCALL_MUTEX: Mutex<()> = Mutex::new(());
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_test_func_hostcall_error(_vmctx: &mut Vmctx) {
            lucet_hostcall_terminate!(ERROR_MESSAGE);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_test_func_hello(vmctx: &mut Vmctx, hello_ptr: u32, hello_len: u32) {
            let heap = vmctx.heap();
            let hello = heap.as_ptr() as usize + hello_ptr as usize;
            if !vmctx.check_heap(hello as *const c_void, hello_len as usize) {
                lucet_hostcall_terminate!("heap access");
            }
            let hello =
                unsafe { std::slice::from_raw_parts(hello as *const u8, hello_len as usize) };
            if hello.starts_with(b"hello") {
                *vmctx.get_embed_ctx_mut::<bool>() = true;
            }
        }

        #[lucet_hostcall]
        #[allow(unreachable_code)]
        #[no_mangle]
        pub fn hostcall_test_func_hostcall_error_unwind(_vmctx: &mut Vmctx) {
            let _lock = HOSTCALL_MUTEX.lock().unwrap();
            unsafe {
                lucet_hostcall_terminate!(ERROR_MESSAGE);
            }
            drop(_lock);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_bad_borrow(vmctx: &mut Vmctx) -> bool {
            let heap = vmctx.heap();
            let mut other_heap = vmctx.heap_mut();
            heap[0] == other_heap[0]
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_missing_embed_ctx(vmctx: &mut Vmctx) -> bool {
            struct S {
                x: bool,
            }
            let ctx = vmctx.get_embed_ctx::<S>();
            ctx.x
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_multiple_vmctx(vmctx: &mut Vmctx) -> bool {
            let mut vmctx1 = unsafe { Vmctx::from_raw(vmctx.as_raw()) };
            vmctx1.heap_mut()[0] = 0xAF;
            drop(vmctx1);

            let mut vmctx2 = unsafe { Vmctx::from_raw(vmctx.as_raw()) };
            let res = vmctx2.heap()[0] == 0xAF;
            drop(vmctx2);

            res
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yields(vmctx: &mut Vmctx) {
            vmctx.yield_();
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_expects_5(vmctx: &mut Vmctx) -> u64 {
            vmctx.yield_expecting_val()
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yields_5(vmctx: &mut Vmctx) {
            vmctx.yield_val(5u64);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_facts(vmctx: &mut Vmctx, n: u64) -> u64 {
            fn fact(vmctx: &mut Vmctx, n: u64) -> u64 {
                let result = if n <= 1 { 1 } else { n * fact(vmctx, n - 1) };
                vmctx.yield_val(result);
                result
            }
            fact(vmctx, n)
        }

        pub enum CoopFactsK {
            Mult(u64, u64),
            Result(u64),
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_coop_facts(vmctx: &mut Vmctx, n: u64) -> u64 {
            fn fact(vmctx: &mut Vmctx, n: u64) -> u64 {
                let result = if n <= 1 {
                    1
                } else {
                    let n_rec = fact(vmctx, n - 1);
                    vmctx.yield_val_expecting_val(CoopFactsK::Mult(n, n_rec))
                };
                vmctx.yield_val(CoopFactsK::Result(result));
                result
            }
            fact(vmctx, n)
        }

        #[test]
        fn instantiate_trivial() {
            let module = test_module_c("host", "trivial.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let inst = region
                .new_instance(module)
                .expect("instance can be created");
        }

        #[test]
        fn run_trivial() {
            let module = test_module_c("host", "trivial.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");
            inst.run("main", &[0u32.into(), 0i32.into()])
                .expect("instance runs");
        }

        #[test]
        fn run_hello() {
            let module = test_module_c("host", "hello.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut inst = region
                .new_instance_builder(module)
                .with_embed_ctx(false)
                .build()
                .expect("instance can be created");

            inst.run("main", &[0u32.into(), 0i32.into()])
                .expect("instance runs");

            assert!(*inst.get_embed_ctx::<bool>().unwrap().unwrap());
        }

        #[test]
        fn run_hostcall_error() {
            let module = test_module_c("host", "hostcall_error.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("main", &[0u32.into(), 0i32.into()]) {
                Err(Error::RuntimeTerminated(term)) => {
                    assert_eq!(
                        *term
                            .provided_details()
                            .expect("user provided termination reason")
                            .downcast_ref::<&'static str>()
                            .expect("error was static str"),
                        ERROR_MESSAGE
                    );
                }
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn run_hostcall_error_unwind() {
            let module =
                test_module_c("host", "hostcall_error_unwind.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("main", &[0u32.into(), 0u32.into()]) {
                Err(Error::RuntimeTerminated(term)) => {
                    assert_eq!(
                        *term
                            .provided_details()
                            .expect("user provided termination reason")
                            .downcast_ref::<&'static str>()
                            .expect("error was static str"),
                        ERROR_MESSAGE
                    );
                }
                res => panic!("unexpected result: {:?}", res),
            }

            assert!(HOSTCALL_MUTEX.is_poisoned());
        }

        #[test]
        fn run_fpe() {
            let module = test_module_c("host", "fpe.c").expect("build and load module");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("trigger_div_error", &[0u32.into()]) {
                Err(Error::RuntimeFault(details)) => {
                    assert_eq!(details.trapcode, Some(TrapCode::IntegerDivByZero));
                }
                res => {
                    panic!("unexpected result: {:?}", res);
                }
            }
        }

        #[test]
        fn run_hostcall_bad_borrow() {
            extern "C" {
                fn hostcall_bad_borrow(vmctx: *mut lucet_vmctx) -> bool;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) {
                hostcall_bad_borrow(vmctx);
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("f", &[]) {
                Err(Error::RuntimeTerminated(details)) => {
                    assert_eq!(details, TerminationDetails::BorrowError("heap_mut"));
                }
                res => {
                    panic!("unexpected result: {:?}", res);
                }
            }
        }

        #[test]
        fn run_hostcall_missing_embed_ctx() {
            extern "C" {
                fn hostcall_missing_embed_ctx(vmctx: *mut lucet_vmctx) -> bool;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) {
                hostcall_missing_embed_ctx(vmctx);
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("f", &[]) {
                Err(Error::RuntimeTerminated(details)) => {
                    assert_eq!(details, TerminationDetails::CtxNotFound);
                }
                res => {
                    panic!("unexpected result: {:?}", res);
                }
            }
        }

        #[test]
        fn run_hostcall_multiple_vmctx() {
            extern "C" {
                fn hostcall_multiple_vmctx(vmctx: *mut lucet_vmctx) -> bool;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) {
                hostcall_multiple_vmctx(vmctx);
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("f", &[])
                .expect("instance runs")
                .expect_returned("instance returned");
            assert_eq!(bool::from(retval), true);
        }

        #[test]
        fn run_hostcall_yields_5() {
            extern "C" {
                fn hostcall_yields_5(vmctx: *mut lucet_vmctx);
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) {
                hostcall_yields_5(vmctx);
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            assert_eq!(
                *inst
                    .run("f", &[])
                    .unwrap()
                    .unwrap_yielded()
                    .downcast::<u64>()
                    .unwrap(),
                5u64
            );
        }

        #[test]
        fn run_hostcall_yield_expects_5() {
            extern "C" {
                fn hostcall_yield_expects_5(vmctx: *mut lucet_vmctx) -> u64;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) -> u64 {
                hostcall_yield_expects_5(vmctx)
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            assert!(inst.run("f", &[]).unwrap().unwrap_yielded().is_none());

            let retval = inst
                .resume_with_val(5u64)
                .expect("instance resumes")
                .unwrap_returned();
            assert_eq!(u64::from(retval), 5u64);
        }

        #[test]
        fn yield_factorials() {
            extern "C" {
                fn hostcall_yield_facts(vmctx: *mut lucet_vmctx, n: u64) -> u64;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) -> u64 {
                hostcall_yield_facts(vmctx, 5)
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let mut facts = vec![];

            let mut res = inst.run("f", &[]).unwrap();

            while res.is_yielded() {
                facts.push(*res.unwrap_yielded().downcast::<u64>().unwrap());
                res = inst.resume().unwrap();
            }

            assert_eq!(facts.as_slice(), &[1, 2, 6, 24, 120]);
            assert_eq!(u64::from(res.unwrap_returned()), 120u64);
        }

        #[test]
        fn coop_factorials() {
            extern "C" {
                fn hostcall_coop_facts(vmctx: *mut lucet_vmctx, n: u64) -> u64;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) -> u64 {
                hostcall_coop_facts(vmctx, 5)
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let mut facts = vec![];

            let mut res = inst.run("f", &[]).unwrap();

            while let Ok(val) = res.yielded_ref() {
                if let Some(k) = val.downcast_ref::<CoopFactsK>() {
                    match k {
                        CoopFactsK::Mult(n, n_rec) => {
                            // guest wants us to multiply for it
                            res = inst.resume_with_val(n * n_rec).unwrap();
                        }
                        CoopFactsK::Result(n) => {
                            // guest is returning an answer
                            facts.push(*n);
                            res = inst.resume().unwrap();
                        }
                    }
                } else {
                    panic!("didn't yield with expected type");
                }
            }

            assert_eq!(facts.as_slice(), &[1, 2, 6, 24, 120]);
            assert_eq!(u64::from(res.unwrap_returned()), 120u64);
        }

        #[test]
        fn resume_unexpected() {
            extern "C" {
                fn hostcall_yields_5(vmctx: *mut lucet_vmctx);
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) {
                hostcall_yields_5(vmctx);
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            assert_eq!(
                *inst
                    .run("f", &[])
                    .unwrap()
                    .unwrap_yielded()
                    .downcast::<u64>()
                    .unwrap(),
                5u64
            );

            match inst.resume_with_val(5u64) {
                Err(Error::InvalidArgument(_)) => (),
                Err(e) => panic!("unexpected error: {}", e),
                Ok(_) => panic!("unexpected success"),
            }
        }

        #[test]
        fn missing_resume_val() {
            extern "C" {
                fn hostcall_yield_expects_5(vmctx: *mut lucet_vmctx) -> u64;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) -> u64 {
                hostcall_yield_expects_5(vmctx)
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            assert!(inst.run("f", &[]).unwrap().unwrap_yielded().is_none());

            match inst.resume() {
                Err(Error::InvalidArgument(_)) => (),
                Err(e) => panic!("unexpected error: {}", e),
                Ok(_) => panic!("unexpected success"),
            }
        }

        #[test]
        fn resume_wrong_type() {
            extern "C" {
                fn hostcall_yield_expects_5(vmctx: *mut lucet_vmctx) -> u64;
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) -> u64 {
                hostcall_yield_expects_5(vmctx)
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            assert!(inst.run("f", &[]).unwrap().unwrap_yielded().is_none());

            match inst.resume_with_val(true) {
                Err(Error::InvalidArgument(_)) => (),
                Err(e) => panic!("unexpected error: {}", e),
                Ok(_) => panic!("unexpected success"),
            }
        }

        /// This test shows that we can send an `InstanceHandle` to another thread while a guest is
        /// yielded, and it resumes successfully.
        #[test]
        fn switch_threads_resume() {
            extern "C" {
                fn hostcall_yields_5(vmctx: *mut lucet_vmctx);
            }

            unsafe extern "C" fn f(vmctx: *mut lucet_vmctx) -> u64 {
                hostcall_yields_5(vmctx);
                42
            }

            let module = MockModuleBuilder::new()
                .with_export_func(MockExportBuilder::new(
                    "f",
                    FunctionPointer::from_usize(f as usize),
                ))
                .build();

            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // make sure we yield with 5 on the original thread
            assert_eq!(
                *inst
                    .run("f", &[])
                    .unwrap()
                    .unwrap_yielded()
                    .downcast::<u64>()
                    .unwrap(),
                5u64
            );

            let res = std::thread::spawn(move || {
                // but then move the instance to another thread and resume it from there
                inst.resume()
                    .expect("instance resumes")
                    .returned()
                    .expect("returns 42")
            })
            .join()
            .unwrap();
            assert_eq!(u64::from(res), 42u64);
        }
    };
}
