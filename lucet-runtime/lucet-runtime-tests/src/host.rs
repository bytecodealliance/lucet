#[macro_export]
macro_rules! host_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {
        use lazy_static::lazy_static;
        use libc::c_void;
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, lucet_hostcall_terminate, DlModule, Error, Limits, Region,
            TerminationDetails, TrapCode,
        };
        use std::cell::RefCell;
        use std::ops::Deref;
        use std::sync::{Arc, Mutex};
        use $crate::build::test_module_c;
        use $crate::helpers::{FunctionPointer, HeapSpec, MockExportBuilder, MockModuleBuilder};

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
            static ref NESTED_OUTER: Mutex<()> = Mutex::new(());
            static ref NESTED_INNER: Mutex<()> = Mutex::new(());
            static ref NESTED_REGS_OUTER: Mutex<()> = Mutex::new(());
            static ref NESTED_REGS_INNER: Mutex<()> = Mutex::new(());
        }

        static mut HOSTCALL_MUTEX: Option<Mutex<()>> = None;
        static mut BAD_ACCESS_UNWIND: Option<Mutex<()>> = None;
        static mut STACK_OVERFLOW_UNWIND: Option<Mutex<()>> = None;

        #[allow(unreachable_code)]
        #[inline]
        unsafe fn unwind_inner(vmctx: &Vmctx, mutex: &Mutex<()>) {
            let lock = mutex.lock().unwrap();
            lucet_hostcall_terminate!(ERROR_MESSAGE);
            drop(lock);
        }

        #[inline]
        unsafe fn unwind_outer(vmctx: &Vmctx, mutex: &Mutex<()>, cb_idx: u32) -> u64 {
            let lock = mutex.lock().unwrap();
            let func = vmctx
                .get_func_from_idx(0, cb_idx)
                .expect("can get function by index");
            let func = std::mem::transmute::<usize, extern "C" fn(*const lucet_vmctx) -> u64>(
                func.ptr.as_usize(),
            );
            let res = (func)(vmctx.as_raw());
            drop(lock);
            res
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_test_func_hostcall_error(_vmctx: &Vmctx) {
            lucet_hostcall_terminate!(ERROR_MESSAGE);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_test_func_hello(vmctx: &Vmctx, hello_ptr: u32, hello_len: u32) {
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
        pub fn hostcall_test_func_hostcall_error_unwind(vmctx: &Vmctx) {
            let lock = unsafe { HOSTCALL_MUTEX.as_ref().unwrap() }.lock().unwrap();
            unsafe {
                lucet_hostcall_terminate!(ERROR_MESSAGE);
            }
            drop(lock);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn nested_error_unwind_outer(
            vmctx: &Vmctx,
            cb_idx: u32,
        ) -> u64 {
            unsafe {
                unwind_outer(vmctx, &*NESTED_OUTER, cb_idx)
            }
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn nested_error_unwind_inner(
            vmctx: &Vmctx,
        ) -> () {
            unsafe {
                unwind_inner(vmctx, &*NESTED_INNER)
            }
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn nested_error_unwind_regs_outer(
            vmctx: &Vmctx,
            cb_idx: u32,
        ) -> u64 {
            unsafe {
                unwind_outer(vmctx, &*NESTED_REGS_OUTER, cb_idx)
            }
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn nested_error_unwind_regs_inner(
            vmctx: &Vmctx,
        ) -> () {
            unsafe {
                unwind_inner(vmctx, &*NESTED_REGS_INNER)
            }
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_panic(
            _vmctx: &Vmctx,
        ) -> () {
            panic!("hostcall_panic");
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_restore_callee_saved(
            vmctx: &Vmctx,
            cb_idx: u32,
        ) -> u64 {
            let mut a: u64;
            let mut b: u64 = 0xAAAAAAAA00000001;
            let mut c: u64 = 0xAAAAAAAA00000002;
            let mut d: u64 = 0xAAAAAAAA00000003;
            let mut e: u64 = 0xAAAAAAAA00000004;
            let mut f: u64 = 0xAAAAAAAA00000005;
            let mut g: u64 = 0xAAAAAAAA00000006;
            let mut h: u64 = 0xAAAAAAAA00000007;
            let mut i: u64 = 0xAAAAAAAA00000008;
            let mut j: u64 = 0xAAAAAAAA00000009;
            let mut k: u64 = 0xAAAAAAAA0000000A;
            let mut l: u64 = 0xAAAAAAAA0000000B;

            a = b.wrapping_add(c ^ 0);
            b = c.wrapping_add(d ^ 1);
            c = d.wrapping_add(e ^ 2);
            d = e.wrapping_add(f ^ 3);
            e = f.wrapping_add(g ^ 4);
            f = g.wrapping_add(h ^ 5);
            g = h.wrapping_add(i ^ 6);
            h = i.wrapping_add(j ^ 7);
            i = j.wrapping_add(k ^ 8);
            j = k.wrapping_add(l ^ 9);
            k = l.wrapping_add(a ^ 10);
            l = a.wrapping_add(b ^ 11);

            let func = vmctx
                .get_func_from_idx(0, cb_idx)
                .expect("can get function by index");
            let func = unsafe {
                std::mem::transmute::<usize, extern "C" fn(*const lucet_vmctx) -> u64>(
                    func.ptr.as_usize(),
                )
            };
            let vmctx_raw = vmctx.as_raw();
            let res = std::panic::catch_unwind(|| {
                (func)(vmctx_raw);
            });
            assert!(res.is_err());

            a = b.wrapping_mul(c & 0);
            b = c.wrapping_mul(d & 1);
            c = d.wrapping_mul(e & 2);
            d = e.wrapping_mul(f & 3);
            e = f.wrapping_mul(g & 4);
            f = g.wrapping_mul(h & 5);
            g = h.wrapping_mul(i & 6);
            h = i.wrapping_mul(j & 7);
            i = j.wrapping_mul(k & 8);
            j = k.wrapping_mul(l & 9);
            k = l.wrapping_mul(a & 10);
            l = a.wrapping_mul(b & 11);

            a ^ b ^ c ^ d ^ e ^ f ^ g ^ h ^ i ^ j ^ k ^ l
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_stack_overflow_unwind(
            vmctx: &Vmctx,
            cb_idx: u32,
        ) -> () {
            let lock = unsafe { STACK_OVERFLOW_UNWIND.as_ref().unwrap() }.lock().unwrap();

            let func = vmctx
                .get_func_from_idx(0, cb_idx)
                .expect("can get function by index");
            let func = unsafe {
                std::mem::transmute::<usize, extern "C" fn(*const lucet_vmctx)>(
                    func.ptr.as_usize(),
                )
            };
            let vmctx_raw = vmctx.as_raw();
            func(vmctx_raw);

            drop(lock);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_bad_access_unwind(
            vmctx: &Vmctx,
            cb_idx: u32,
        ) -> () {
            let lock = unsafe { BAD_ACCESS_UNWIND.as_ref().unwrap() }.lock().unwrap();

            let func = vmctx
                .get_func_from_idx(0, cb_idx)
                .expect("can get function by index");
            let func = unsafe {
                std::mem::transmute::<usize, extern "C" fn(*const lucet_vmctx)>(
                    func.ptr.as_usize(),
                )
            };
            let vmctx_raw = vmctx.as_raw();
            func(vmctx_raw);

            drop(lock);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_bad_borrow(vmctx: &Vmctx) -> bool {
            let heap = vmctx.heap();
            let mut other_heap = vmctx.heap_mut();
            heap[0] == other_heap[0]
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_missing_embed_ctx(vmctx: &Vmctx) -> bool {
            struct S {
                x: bool,
            }
            let ctx = vmctx.get_embed_ctx::<S>();
            ctx.x
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_multiple_vmctx(vmctx: &Vmctx) -> bool {
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
        pub fn hostcall_yields(vmctx: &Vmctx) {
            vmctx.yield_();
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_expects_5(vmctx: &Vmctx) -> u64 {
            vmctx.yield_expecting_val()
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yields_5(vmctx: &Vmctx) {
            vmctx.yield_val(5u64);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_facts(vmctx: &Vmctx, n: u64) -> u64 {
            fn fact(vmctx: &Vmctx, n: u64) -> u64 {
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
        pub fn hostcall_coop_facts(vmctx: &Vmctx, n: u64) -> u64 {
            fn fact(vmctx: &Vmctx, n: u64) -> u64 {
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

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_with_borrowed_heap(vmctx: &Vmctx) {
            let heap = vmctx.heap();
            vmctx.yield_val(5u64);
            // shouldn't get here
            assert_eq!(heap[0], 0);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_with_borrowed_globals(vmctx: &Vmctx) {
            let globals = vmctx.globals();
            vmctx.yield_val(5u64);
            // shouldn't get here
            assert_eq!(unsafe { globals[0].i_64 }, 0);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_yield_with_borrowed_ctx(vmctx: &Vmctx) {
            let ctx = vmctx.get_embed_ctx::<u32>();
            vmctx.yield_val(5u64);
            // shouldn't get here
            assert_eq!(ctx.deref(), &0);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_grow_with_borrowed_ctx(vmctx: &Vmctx) {
            let ctx = vmctx.get_embed_ctx::<u32>();
            vmctx.grow_memory(1).expect("grow_memory succeeds");
            assert_eq!(ctx.deref(), &0);
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_grow_with_borrowed_heap(vmctx: &Vmctx) {
            let heap = vmctx.heap();
            vmctx.grow_memory(1).expect("grow_memory succeeds");
            // shouldn't get here
            assert_eq!(heap[0], 0);
        }

        $(
            mod $region_id {
                use lazy_static::lazy_static;
                use libc::c_void;
                use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
                use lucet_runtime::{
                    lucet_hostcall, lucet_hostcall_terminate, DlModule, Error, Limits, Region,
                    RegionCreate, TerminationDetails, TrapCode,
                };
                use std::sync::{Arc, Mutex};
                use $crate::build::test_module_c;
                use $crate::helpers::{FunctionPointer, HeapSpec, MockExportBuilder, MockModuleBuilder, test_ex};
                use $TestRegion as TestRegion;

                #[test]
                fn load_module() {
                    let _module = test_module_c("host", "trivial.c").expect("build and load module");
                }

                #[test]
                fn load_nonexistent_module() {
                    let module = DlModule::load("/non/existient/file");
                    assert!(module.is_err());
                }

                #[test]
                fn instantiate_trivial() {
                    let module = test_module_c("host", "trivial.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let inst = region
                        .new_instance(module)
                        .expect("instance can be created");
                }

                #[test]
                fn run_trivial() {
                    let module = test_module_c("host", "trivial.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");
                    inst.run("main", &[0u32.into(), 0i32.into()])
                        .expect("instance runs");
                }

                #[test]
                fn run_hello() {
                    let module = test_module_c("host", "hello.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");

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
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                                super::ERROR_MESSAGE
                            );
                        }
                        res => panic!("unexpected result: {:?}", res),
                    }
                }

                #[test]
                fn run_hostcall_error_unwind() {
                    test_ex(|| {
                        // Since `hostcall_test_func_hostcall_error_unwind` is reused in two
                        // different modules, meaning two different tests, we need to reset the
                        // mutex it will (hopefully) poison before running this test.
                        //
                        // The contention for this global mutex is why this test must be `test_ex`.
                        unsafe {
                            super::HOSTCALL_MUTEX = Some(Mutex::new(()));
                        }

                        let module =
                            test_module_c("host", "hostcall_error_unwind.c").expect("build and load module");
                        let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");

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
                                    super::ERROR_MESSAGE
                                );
                            }
                            res => panic!("unexpected result: {:?}", res),
                        }

                        unsafe {
                            assert!(super::HOSTCALL_MUTEX.as_ref().unwrap().is_poisoned());
                        }
                    })
                }

                /// Check that if two segments of hostcall stack are present when terminating, that they
                /// both get properly unwound.
                ///
                /// Currently ignored as we don't allow nested hostcalls - the nested hostcall runs afoul
                /// of timeouts' domain-checking logic, which assumes beginning a hostscall will only
                /// happen from a guest context, but when initiated from a nested hostcall is actually a
                /// hostcall context
                #[test]
                #[ignore]
                fn nested_error_unwind() {
                    let module =
                        test_module_c("host", "nested_error_unwind.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    match inst.run("entrypoint", &[]) {
                        Err(Error::RuntimeTerminated(term)) => {
                            assert_eq!(
                                *term
                                    .provided_details()
                                    .expect("user provided termination reason")
                                    .downcast_ref::<&'static str>()
                                    .expect("error was static str"),
                                super::ERROR_MESSAGE
                            );
                        }
                        res => panic!("unexpected result: {:?}", res),
                    }

                    assert!(super::NESTED_OUTER.is_poisoned());
                    assert!(super::NESTED_INNER.is_poisoned());
                }

                /// Like `nested_error_unwind`, but the guest code callback in between the two segments of
                /// hostcall stack uses enough locals to require saving callee registers.
                ///
                /// Currently ignored as we don't allow nested hostcalls - the nested hostcall runs afoul
                /// of timeouts' domain-checking logic, which assumes beginning a hostscall will only
                /// happen from a guest context, but when initiated from a nested hostcall is actually a
                /// hostcall context
                #[test]
                #[ignore]
                fn nested_error_unwind_regs() {
                    let module =
                        test_module_c("host", "nested_error_unwind.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    match inst.run("entrypoint_regs", &[]) {
                        Err(Error::RuntimeTerminated(term)) => {
                            assert_eq!(
                                *term
                                    .provided_details()
                                    .expect("user provided termination reason")
                                    .downcast_ref::<&'static str>()
                                    .expect("error was static str"),
                                super::ERROR_MESSAGE
                            );
                        }
                        res => panic!("unexpected result: {:?}", res),
                    }

                    assert!(super::NESTED_REGS_OUTER.is_poisoned());
                    assert!(super::NESTED_REGS_INNER.is_poisoned());
                }

                /// Ensures that callee-saved registers are properly restored following a `catch_unwind`
                /// that catches a panic.
                ///
                /// Currently ignored as we don't allow nested hostcalls - the nested hostcall runs afoul
                /// of timeouts' domain-checking logic, which assumes beginning a hostscall will only
                /// happen from a guest context, but when initiated from a nested hostcall is actually a
                /// hostcall context
                #[ignore]
                #[test]
                fn restore_callee_saved() {
                    let module =
                        test_module_c("host", "nested_error_unwind.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");
                    assert_eq!(
                        u64::from(inst.run("entrypoint_restore", &[]).unwrap().unwrap_returned()),
                        6148914668330025056
                    );
                }

                /// Ensures that hostcall stack frames get unwound when a fault occurs in guest code.
                #[test]
                fn bad_access_unwind() {
                    test_ex(|| {
                        unsafe {
                            super::BAD_ACCESS_UNWIND = Some(Mutex::new(()));
                        }
                        let module = test_module_c("host", "fault_unwind.c").expect("build and load module");
                        let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");
                        inst.run("bad_access", &[]).unwrap_err();
                        inst.reset().unwrap();
                        unsafe {
                            assert!(unsafe { super::BAD_ACCESS_UNWIND.as_ref().unwrap() }.is_poisoned());
                        }
                    })
                }

                /// Ensures that hostcall stack frames get unwound even when a stack overflow occurs in
                /// guest code.
                #[test]
                fn stack_overflow_unwind() {
                    test_ex(|| {
                        unsafe {
                            super::STACK_OVERFLOW_UNWIND = Some(Mutex::new(()));
                        }
                        let module = test_module_c("host", "fault_unwind.c").expect("build and load module");
                        let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                        let mut inst = region
                            .new_instance(module)
                            .expect("instance can be created");
                        inst.run("stack_overflow", &[]).unwrap_err();
                        inst.reset().unwrap();
                        assert!(unsafe { super::STACK_OVERFLOW_UNWIND.as_ref().unwrap() }.is_poisoned());
                    })
                }

                #[test]
                fn run_fpe() {
                    let module = test_module_c("host", "fpe.c").expect("build and load module");
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_bad_borrow(vmctx: *const lucet_vmctx) -> bool;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_bad_borrow(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_missing_embed_ctx(vmctx: *const lucet_vmctx) -> bool;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_missing_embed_ctx(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_multiple_vmctx(vmctx: *const lucet_vmctx) -> bool;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_multiple_vmctx(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_yields_5(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_yields_5(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_yield_expects_5(vmctx: *const lucet_vmctx) -> u64;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) -> u64 {
                        hostcall_yield_expects_5(vmctx)
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_yield_facts(vmctx: *const lucet_vmctx, n: u64) -> u64;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) -> u64 {
                        hostcall_yield_facts(vmctx, 5)
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_coop_facts(vmctx: *const lucet_vmctx, n: u64) -> u64;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) -> u64 {
                        hostcall_coop_facts(vmctx, 5)
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    let mut facts = vec![];

                    let mut res = inst.run("f", &[]).unwrap();

                    while let Ok(val) = res.yielded_ref() {
                        if let Some(k) = val.downcast_ref::<super::CoopFactsK>() {
                            match k {
                                super::CoopFactsK::Mult(n, n_rec) => {
                                    // guest wants us to multiply for it
                                    res = inst.resume_with_val(n * n_rec).unwrap();
                                }
                                super::CoopFactsK::Result(n) => {
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
                        fn hostcall_yields_5(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_yields_5(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_yield_expects_5(vmctx: *const lucet_vmctx) -> u64;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) -> u64 {
                        hostcall_yield_expects_5(vmctx)
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_yield_expects_5(vmctx: *const lucet_vmctx) -> u64;
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) -> u64 {
                        hostcall_yield_expects_5(vmctx)
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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
                        fn hostcall_yields_5(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) -> u64 {
                        hostcall_yields_5(vmctx);
                        42
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
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

                #[test]
                fn yield_with_borrowed_heap_terminates() {
                    extern "C" {
                        fn hostcall_yield_with_borrowed_heap(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_yield_with_borrowed_heap(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    match inst.run("f", &[]) {
                        Err(Error::RuntimeTerminated(details)) => {
                            assert_eq!(details, TerminationDetails::BorrowError("heap"));
                        }
                        res => {
                            panic!("unexpected result: {:?}", res);
                        }
                    }
                }

                #[test]
                fn yield_with_borrowed_globals_terminates() {
                    extern "C" {
                        fn hostcall_yield_with_borrowed_globals(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_yield_with_borrowed_globals(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    match inst.run("f", &[]) {
                        Err(Error::RuntimeTerminated(details)) => {
                            assert_eq!(details, TerminationDetails::BorrowError("globals"));
                        }
                        res => {
                            panic!("unexpected result: {:?}", res);
                        }
                    }
                }

                #[test]
                fn yield_with_borrowed_ctx_terminates() {
                    extern "C" {
                        fn hostcall_yield_with_borrowed_ctx(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_yield_with_borrowed_ctx(vmctx);
                    }

                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.insert_embed_ctx(0u32);

                    match inst.run("f", &[]) {
                        Err(Error::RuntimeTerminated(details)) => {
                            assert_eq!(details, TerminationDetails::BorrowError("embed_ctx"));
                        }
                        res => {
                            panic!("unexpected result: {:?}", res);
                        }
                    }
                }

                #[test]
                fn grow_with_borrowed_ctx() {
                    extern "C" {
                        fn hostcall_grow_with_borrowed_ctx(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_grow_with_borrowed_ctx(vmctx);
                    }

                    const HEAP_SPEC: HeapSpec = HeapSpec {
                        reserved_size: 4 * 1024 * 1024,
                        guard_size: 4 * 1024 * 1024,
                        initial_size: 64 * 1024,
                        max_size: Some(2 * 64 * 1024),
                    };
                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .with_heap_spec(HEAP_SPEC)
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    inst.insert_embed_ctx(0u32);

                    inst.run("f", &[]).expect("instance runs");
                }

                #[test]
                fn grow_with_borrowed_heap_terminates() {
                    extern "C" {
                        fn hostcall_grow_with_borrowed_heap(vmctx: *const lucet_vmctx);
                    }

                    unsafe extern "C" fn f(vmctx: *const lucet_vmctx) {
                        hostcall_grow_with_borrowed_heap(vmctx);
                    }

                    const HEAP_SPEC: HeapSpec = HeapSpec {
                        reserved_size: 4 * 1024 * 1024,
                        guard_size: 4 * 1024 * 1024,
                        initial_size: 64 * 1024,
                        max_size: Some(2 * 64 * 1024),
                    };
                    let module = MockModuleBuilder::new()
                        .with_export_func(MockExportBuilder::new(
                            "f",
                            FunctionPointer::from_usize(f as usize),
                        ))
                        .with_heap_spec(HEAP_SPEC)
                        .build();

                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    match inst.run("f", &[]) {
                        Err(Error::RuntimeTerminated(details)) => {
                            assert_eq!(details, TerminationDetails::BorrowError("heap"));
                        }
                        res => {
                            panic!("unexpected result: {:?}", res);
                        }
                    }
                }
            }
        )*

        #[test]
        fn ensure_linked() {
            lucet_runtime::lucet_internal_ensure_linked();
        }
    };
}
