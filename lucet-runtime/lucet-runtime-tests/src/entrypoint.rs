use crate::build::test_module_wasm;
use crate::helpers::{MockExportBuilder, MockModuleBuilder};
use lucet_module::{lucet_signature, FunctionPointer};
use lucet_runtime_internals::module::Module;
use lucet_runtime_internals::vmctx::lucet_vmctx;
use std::sync::Arc;

pub fn wat_calculator_module() -> Arc<dyn Module> {
    test_module_wasm("entrypoint", "calculator.wat").expect("build and load module")
}

pub fn mock_calculator_module() -> Arc<dyn Module> {
    extern "C" fn add_2(_vmctx: *mut lucet_vmctx, arg0: u64, arg1: u64) -> u64 {
        arg0 + arg1
    }

    extern "C" fn add_4_hostcall(
        _vmctx: *mut lucet_vmctx,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
    ) -> u64 {
        arg0 + arg1 + arg2 + arg3
    }

    extern "C" fn add_10(
        _vmctx: *mut lucet_vmctx,
        arg0: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
        arg6: u64,
        arg7: u64,
        arg8: u64,
        arg9: u64,
    ) -> u64 {
        arg0 + arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8 + arg9
    }

    extern "C" fn mul_2(_vmctx: *mut lucet_vmctx, arg0: u64, arg1: u64) -> u64 {
        arg0 * arg1
    }

    extern "C" fn add_f32_2(_vmctx: *mut lucet_vmctx, arg0: f32, arg1: f32) -> f32 {
        arg0 + arg1
    }

    extern "C" fn add_f64_2(_vmctx: *mut lucet_vmctx, arg0: f64, arg1: f64) -> f64 {
        arg0 + arg1
    }

    extern "C" fn add_f32_10(
        _vmctx: *mut lucet_vmctx,
        arg0: f32,
        arg1: f32,
        arg2: f32,
        arg3: f32,
        arg4: f32,
        arg5: f32,
        arg6: f32,
        arg7: f32,
        arg8: f32,
        arg9: f32,
    ) -> f32 {
        arg0 + arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8 + arg9
    }

    extern "C" fn add_f64_10(
        _vmctx: *mut lucet_vmctx,
        arg0: f64,
        arg1: f64,
        arg2: f64,
        arg3: f64,
        arg4: f64,
        arg5: f64,
        arg6: f64,
        arg7: f64,
        arg8: f64,
        arg9: f64,
    ) -> f64 {
        arg0 + arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8 + arg9
    }

    extern "C" fn add_mixed_20(
        _vmctx: *mut lucet_vmctx,
        arg0: f64,
        arg1: u8,
        arg2: f32,
        arg3: f64,
        arg4: u16,
        arg5: f32,
        arg6: f64,
        arg7: u32,
        arg8: f32,
        arg9: f64,
        arg10: bool,
        arg11: f32,
        arg12: f64,
        arg13: std::os::raw::c_int,
        arg14: f32,
        arg15: f64,
        arg16: std::os::raw::c_long,
        arg17: f32,
        arg18: f64,
        arg19: std::os::raw::c_longlong,
    ) -> f64 {
        arg0 as f64
            + arg1 as f64
            + arg2 as f64
            + arg3 as f64
            + arg4 as f64
            + arg5 as f64
            + arg6 as f64
            + arg7 as f64
            + arg8 as f64
            + arg9 as f64
            + if arg10 { 1.0f64 } else { 0.0f64 }
            + arg11 as f64
            + arg12 as f64
            + arg13 as f64
            + arg14 as f64
            + arg15 as f64
            + arg16 as f64
            + arg17 as f64
            + arg18 as f64
            + arg19 as f64
    }

    MockModuleBuilder::new()
        .with_export_func(
            MockExportBuilder::new("add_2", FunctionPointer::from_usize(add_2 as usize))
                .with_sig(lucet_signature!((I64, I64) -> I64)),
        )
        .with_exported_import_func(
            "add_4_reexport",
            FunctionPointer::from_usize(add_4_hostcall as usize),
            lucet_signature!((I64, I64, I64, I64) -> I64),
        )
        .with_export_func(
            MockExportBuilder::new("add_10", FunctionPointer::from_usize(add_10 as usize))
                .with_sig(lucet_signature!(
                    (I64, I64, I64, I64, I64, I64, I64, I64, I64, I64) -> I64)),
        )
        .with_export_func(
            MockExportBuilder::new("mul_2", FunctionPointer::from_usize(mul_2 as usize))
                .with_sig(lucet_signature!((I64, I64) -> I64)),
        )
        .with_export_func(
            MockExportBuilder::new("add_f32_2", FunctionPointer::from_usize(add_f32_2 as usize))
                .with_sig(lucet_signature!((F32, F32) -> F32)),
        )
        .with_export_func(
            MockExportBuilder::new("add_f64_2", FunctionPointer::from_usize(add_f64_2 as usize))
                .with_sig(lucet_signature!((F64, F64) -> F64)),
        )
        .with_export_func(
            MockExportBuilder::new(
                "add_f32_10",
                FunctionPointer::from_usize(add_f32_10 as usize),
            )
            .with_sig(lucet_signature!(
                    (F32, F32, F32, F32, F32, F32, F32, F32, F32, F32) -> F32)),
        )
        .with_export_func(
            MockExportBuilder::new(
                "add_f64_10",
                FunctionPointer::from_usize(add_f64_10 as usize),
            )
            .with_sig(lucet_signature!(
                    (F64, F64, F64, F64, F64, F64, F64, F64, F64, F64) -> F64)),
        )
        .with_export_func(
            MockExportBuilder::new(
                "add_mixed_20",
                FunctionPointer::from_usize(add_mixed_20 as usize),
            )
            .with_sig(lucet_signature!(
                    (
                        F64, I32, F32, F64, I32, F32,
                        F64, I32, F32, F64, I32, F32,
                        F64, I32, F32, F64, I64, F32,
                        F64, I64
                    ) -> F64)),
        )
        .build()
}

#[macro_export]
macro_rules! entrypoint_tests {
    ( $TestRegion:path ) => {
        use libc::c_void;
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{
            lucet_hostcall, DlModule, Error, Limits, Module, Region, Val, WASM_PAGE_SIZE,
        };
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::entrypoint::{mock_calculator_module, wat_calculator_module};

        #[lucet_hostcall]
        #[no_mangle]
        pub fn black_box(_vmctx: &mut Vmctx, _val: *mut c_void) {}

        #[test]
        fn mock_calc_add_2() {
            calc_add_2(mock_calculator_module())
        }

        #[test]
        fn wat_calc_add_2() {
            calc_add_2(wat_calculator_module());
        }

        fn calc_add_2(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("add_2", &[123u64.into(), 456u64.into()])
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(u64::from(retval), 123u64 + 456);
        }

        #[test]
        fn mock_calc_add_10() {
            calc_add_10(mock_calculator_module())
        }

        #[test]
        fn wat_calc_add_10() {
            calc_add_10(wat_calculator_module())
        }

        fn calc_add_10(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // Add all 10 arguments. Why 10? Because its more than will fit in registers to be passed to
            // `guest_add_10` by liblucet, so it will make sure that the calling convention of putting stuff
            // on the stack is working.
            //
            // A better test might be to use an operation that doesn't commute, so we can verify that the
            // order is correct.
            let retval = inst
                .run(
                    "add_10",
                    &[
                        1u64.into(),
                        2u64.into(),
                        3u64.into(),
                        4u64.into(),
                        5u64.into(),
                        6u64.into(),
                        7u64.into(),
                        8u64.into(),
                        9u64.into(),
                        10u64.into(),
                    ],
                )
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(u64::from(retval), 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10);
        }

        #[test]
        fn mock_calc_mul_2() {
            calc_mul_2(mock_calculator_module())
        }

        #[test]
        fn wat_calc_mul_2() {
            calc_mul_2(wat_calculator_module())
        }

        fn calc_mul_2(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("mul_2", &[123u64.into(), 456u64.into()])
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(u64::from(retval), 123 * 456);
        }

        #[test]
        fn mock_calc_add_then_mul() {
            calc_add_then_mul(mock_calculator_module())
        }

        fn calc_add_then_mul(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("add_2", &[111u64.into(), 222u64.into()])
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(u64::from(retval), 111 + 222);

            let retval = inst
                .run("mul_2", &[333u64.into(), 444u64.into()])
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(u64::from(retval), 333 * 444);
        }

        #[test]
        fn mock_calc_invalid_entrypoint() {
            calc_invalid_entrypoint(mock_calculator_module())
        }

        #[test]
        fn wat_calc_invalid_entrypoint() {
            calc_invalid_entrypoint(wat_calculator_module())
        }

        fn calc_invalid_entrypoint(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("invalid", &[123u64.into(), 456u64.into()]) {
                Err(Error::SymbolNotFound(sym)) => assert_eq!(sym, "invalid"),
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn mock_calc_add_f32_2() {
            calc_add_f32_2(mock_calculator_module());
        }
        #[test]
        fn wat_calc_add_f32_2() {
            calc_add_f32_2(wat_calculator_module());
        }

        fn calc_add_f32_2(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("add_f32_2", &[(-6.9f32).into(), 4.2f32.into()])
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(f32::from(retval), -6.9 + 4.2);
        }

        #[test]
        fn mock_calc_add_f64_2() {
            calc_add_f64_2(mock_calculator_module());
        }
        #[test]
        fn wat_calc_add_f64_2() {
            calc_add_f64_2(wat_calculator_module());
        }
        fn calc_add_f64_2(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("add_f64_2", &[(-6.9f64).into(), 4.2f64.into()])
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(f64::from(retval), -6.9 + 4.2);
        }

        #[test]
        fn mock_calc_add_f32_10() {
            calc_add_f32_10(mock_calculator_module());
        }
        #[test]
        fn wat_calc_add_f32_10() {
            calc_add_f32_10(wat_calculator_module());
        }
        fn calc_add_f32_10(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run(
                    "add_f32_10",
                    &[
                        0.1f32.into(),
                        0.2f32.into(),
                        0.3f32.into(),
                        0.4f32.into(),
                        0.5f32.into(),
                        0.6f32.into(),
                        0.7f32.into(),
                        0.8f32.into(),
                        0.9f32.into(),
                        1.0f32.into(),
                    ],
                )
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(
                f32::from(retval),
                0.1 + 0.2 + 0.3 + 0.4 + 0.5 + 0.6 + 0.7 + 0.8 + 0.9 + 1.0
            );
        }

        #[test]
        fn mock_calc_add_f64_10() {
            calc_add_f64_10(mock_calculator_module());
        }
        #[test]
        fn wat_calc_add_f64_10() {
            calc_add_f64_10(wat_calculator_module());
        }
        fn calc_add_f64_10(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run(
                    "add_f64_10",
                    &[
                        0.1f64.into(),
                        0.2f64.into(),
                        0.3f64.into(),
                        0.4f64.into(),
                        0.5f64.into(),
                        0.6f64.into(),
                        0.7f64.into(),
                        0.8f64.into(),
                        0.9f64.into(),
                        1.0f64.into(),
                    ],
                )
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(
                f64::from(retval),
                0.1 + 0.2 + 0.3 + 0.4 + 0.5 + 0.6 + 0.7 + 0.8 + 0.9 + 1.0
            );
        }

        #[test]
        fn mock_calc_add_mixed_20() {
            calc_add_mixed_20(mock_calculator_module());
        }
        // TODO: it would be pretty annoying to write the mixed_20 calc test in wasm, so we havent.
        fn calc_add_mixed_20(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run(
                    "add_mixed_20",
                    &[
                        (-1.1f64).into(),
                        1u8.into(),
                        2.1f32.into(),
                        3.1f64.into(),
                        4u16.into(),
                        5.1f32.into(),
                        6.1f64.into(),
                        7u32.into(),
                        8.1f32.into(),
                        9.1f64.into(),
                        true.into(),
                        11.1f32.into(),
                        12.1f64.into(),
                        13u32.into(),
                        14.1f32.into(),
                        15.1f64.into(),
                        16u64.into(),
                        17.1f32.into(),
                        18.1f64.into(),
                        19u64.into(),
                    ],
                )
                .expect("instance runs")
                .unwrap_returned();

            assert_eq!(
                f64::from(retval),
                -1.1f64
                    + 1u8 as f64
                    + 2.1f32 as f64
                    + 3.1f64
                    + 4u16 as f64
                    + 5.1f32 as f64
                    + 6.1f64
                    + 7u32 as f64
                    + 8.1f32 as f64
                    + 9.1f64
                    + 1 as f64
                    + 11.1f32 as f64
                    + 12.1f64
                    + 13u32 as f64
                    + 14.1f32 as f64
                    + 15.1f64
                    + 16u64 as f64
                    + 17.1f32 as f64
                    + 18.1f64
                    + 19u64 as f64
            );
        }

        #[test]
        fn mock_typecheck_entrypoint_wrong_args() {
            typecheck_entrypoint_wrong_args(mock_calculator_module())
        }

        #[test]
        fn wat_typecheck_entrypoint_wrong_args() {
            typecheck_entrypoint_wrong_args(wat_calculator_module())
        }

        fn typecheck_entrypoint_wrong_args(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("add_2", &[123.0f64.into(), 456.0f64.into()]) {
                Err(Error::InvalidArgument(err)) => {
                    assert_eq!(err, "entrypoint function signature mismatch")
                }
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn mock_typecheck_entrypoint_too_few_args() {
            typecheck_entrypoint_too_few_args(mock_calculator_module())
        }

        #[test]
        fn wat_typecheck_entrypoint_too_few_args() {
            typecheck_entrypoint_too_few_args(wat_calculator_module())
        }

        fn typecheck_entrypoint_too_few_args(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("add_2", &[123u64.into()]) {
                Err(Error::InvalidArgument(err)) => assert_eq!(
                    err,
                    "entrypoint function signature mismatch (number of arguments is incorrect)"
                ),
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn mock_typecheck_entrypoint_too_many_args() {
            typecheck_entrypoint_too_many_args(mock_calculator_module())
        }

        #[test]
        fn wat_typecheck_entrypoint_too_many_args() {
            typecheck_entrypoint_too_many_args(wat_calculator_module())
        }

        fn typecheck_entrypoint_too_many_args(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("add_2", &[123u64.into(), 456u64.into(), 789u64.into()]) {
                Err(Error::InvalidArgument(err)) => assert_eq!(
                    err,
                    "entrypoint function signature mismatch (number of arguments is incorrect)"
                ),
                res => panic!("unexpected result: {:?}", res),
            }
        }

        #[test]
        fn mock_imported_entrypoint() {
            imported_entrypoint(mock_calculator_module())
        }

        #[test]
        fn wat_imported_entrypoint() {
            imported_entrypoint(wat_calculator_module())
        }

        fn imported_entrypoint(module: Arc<dyn Module>) {
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run(
                    "add_4_reexport",
                    &[123u64.into(), 456u64.into(), 789u64.into(), 432u64.into()],
                )
                .unwrap()
                .unwrap_returned();
            assert_eq!(u64::from(retval), 1800);
        }

        use $crate::build::test_module_c;
        const TEST_REGION_INIT_VAL: libc::c_int = 123;
        const TEST_REGION_SIZE: libc::size_t = 4;

        #[test]
        fn allocator_create_region() {
            use byteorder::{LittleEndian, ReadBytesExt};

            let module =
                test_module_c("entrypoint", "use_allocator.c").expect("module builds and loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // First, we need to get an unused location in linear memory for the pointer that will be passed
            // as an argument to create_and_memset.
            let new_page = inst.grow_memory(1).expect("grow_memory succeeds");
            assert!(new_page > 0);
            // wasm memory index for the start of the new page
            let loc_outval = new_page * WASM_PAGE_SIZE;

            // This function will call `malloc` for the given size, then `memset` the entire region to the
            // init_as argument. The pointer to the allocated region gets stored in loc_outval.
            inst.run(
                "create_and_memset",
                &[
                    // int init_as
                    TEST_REGION_INIT_VAL.into(),
                    // size_t size
                    TEST_REGION_SIZE.into(),
                    // char** ptr_outval
                    Val::GuestPtr(loc_outval),
                ],
            )
            .expect("instance runs");

            // The location of the created region should be in a new page that the allocator grabbed from
            // the runtime. That page will be above the one we got above.
            let heap = inst.heap();
            let loc_region_1 = (&heap[loc_outval as usize..])
                .read_u32::<LittleEndian>()
                .expect("can read outval");
            assert!(loc_region_1 > loc_outval);

            // Each character in the newly created region will match the expected value.
            for i in 0..TEST_REGION_SIZE {
                assert_eq!(
                    TEST_REGION_INIT_VAL as u8,
                    heap[loc_region_1 as usize + i],
                    "character in new region matches"
                );
            }
        }

        #[test]
        fn allocator_create_region_and_increment() {
            use byteorder::{LittleEndian, ReadBytesExt};

            let module =
                test_module_c("entrypoint", "use_allocator.c").expect("module builds and loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // First, we need to get an unused location in linear memory for the pointer that will be passed
            // as an argument to create_and_memset.
            let new_page = inst.grow_memory(1).expect("grow_memory succeeds");
            assert!(new_page > 0);
            // wasm memory index for the start of the new page
            let loc_outval = new_page * WASM_PAGE_SIZE as u32;

            // Create a region and initialize it, just like above
            inst.run(
                "create_and_memset",
                &[
                    // int init_as
                    TEST_REGION_INIT_VAL.into(),
                    // size_t size
                    TEST_REGION_SIZE.into(),
                    // char** ptr_outval
                    Val::GuestPtr(loc_outval),
                ],
            )
            .expect("instance runs");

            // The location of the created region should be in a new page that the allocator grabbed from
            // the runtime. That page will be above the one we got above.
            let heap = inst.heap();
            let loc_region_1 = (&heap[loc_outval as usize..])
                .read_u32::<LittleEndian>()
                .expect("can read outval");
            assert!(loc_region_1 > loc_outval);

            // Each character in the newly created region will match the expected value.
            for i in 0..TEST_REGION_SIZE {
                assert_eq!(
                    TEST_REGION_INIT_VAL as u8,
                    heap[loc_region_1 as usize + i],
                    "character in new region matches"
                );
            }

            // Then increment the first location in the region
            inst.run("increment_ptr", &[Val::GuestPtr(loc_region_1)])
                .expect("instance runs");

            let heap = inst.heap();
            // Just the first location in the region should be incremented
            for i in 0..TEST_REGION_SIZE {
                if i == 0 {
                    assert_eq!(
                        TEST_REGION_INIT_VAL as u8 + 1,
                        heap[loc_region_1 as usize + i],
                        "character in new region matches"
                    );
                } else {
                    assert_eq!(
                        TEST_REGION_INIT_VAL as u8,
                        heap[loc_region_1 as usize + i],
                        "character in new region matches"
                    );
                }
            }
        }

        const TEST_REGION2_INIT_VAL: libc::c_int = 99;
        const TEST_REGION2_SIZE: libc::size_t = 420;

        #[test]
        fn allocator_create_two_regions() {
            use byteorder::{LittleEndian, ReadBytesExt};

            let module =
                test_module_c("entrypoint", "use_allocator.c").expect("module builds and loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // same as above
            let new_page = inst.grow_memory(1).expect("grow_memory succeeds");
            assert!(new_page > 0);
            // wasm memory index for the start of the new page
            let loc_outval = new_page * WASM_PAGE_SIZE;

            // same as above
            inst.run(
                "create_and_memset",
                &[
                    // int init_as
                    TEST_REGION_INIT_VAL.into(),
                    // size_t size
                    TEST_REGION_SIZE.into(),
                    // char** ptr_outval
                    Val::GuestPtr(loc_outval),
                ],
            )
            .expect("instance runs");

            let heap = inst.heap();
            let loc_region_1 = (&heap[loc_outval as usize..])
                .read_u32::<LittleEndian>()
                .expect("can read outval");
            assert!(loc_region_1 > loc_outval);

            // Create a second region
            inst.run(
                "create_and_memset",
                &[
                    // int init_as
                    TEST_REGION2_INIT_VAL.into(),
                    // size_t size
                    TEST_REGION2_SIZE.into(),
                    // char** ptr_outval
                    Val::GuestPtr(loc_outval),
                ],
            )
            .expect("instance runs");

            // The allocator should pick a spot *after* the first region for the second one. (It doesn't
            // have to, but it will.) This shows that the allocator's metadata (free list) is preserved
            // between the runs.
            let heap = inst.heap();
            let loc_region_2 = (&heap[loc_outval as usize..])
                .read_u32::<LittleEndian>()
                .expect("can read outval");
            assert!(loc_region_2 > loc_region_1);

            // After this, both regions should be initialized as expected
            for i in 0..TEST_REGION_SIZE {
                assert_eq!(
                    TEST_REGION_INIT_VAL as u8,
                    heap[loc_region_1 as usize + i],
                    "character in region 1 matches"
                );
            }

            for i in 0..TEST_REGION2_SIZE {
                assert_eq!(
                    TEST_REGION2_INIT_VAL as u8,
                    heap[loc_region_2 as usize + i],
                    "character in region 2 matches"
                );
            }
        }

        #[test]
        fn entrypoint_ctype() {
            use byteorder::{LittleEndian, ReadBytesExt};
            let module = test_module_c("entrypoint", "ctype.c").expect("module builds and loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            // First, we need to get an unused location in linear memory for the pointer that will be passed
            // as an argument to create_and_memset.
            let new_page = inst.grow_memory(1).expect("grow_memory succeeds");
            assert!(new_page > 0);
            // wasm memory index for the start of the new page
            let loc_ctxstar = new_page * WASM_PAGE_SIZE;

            // Run the setup routine
            inst.run(
                "ctype_setup",
                &[
                    std::ptr::null::<c_void>().into(),
                    Val::GuestPtr(loc_ctxstar),
                ],
            )
            .expect("instance runs");

            // Grab the value of the pointer that the setup routine wrote
            let heap = inst.heap();
            let ctxstar = (&heap[loc_ctxstar as usize..])
                .read_u32::<LittleEndian>()
                .expect("can read ctxstar");
            assert!(ctxstar > 0);

            // Run the body routine
            inst.run("ctype_body", &[Val::GuestPtr(ctxstar)])
                .expect("instance runs");
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub unsafe extern "C" fn callback_hostcall(vmctx: &mut Vmctx, cb_idx: u32, x: u64) -> u64 {
            let func = vmctx
                .get_func_from_idx(0, cb_idx)
                .expect("can get function by index");
            let func = std::mem::transmute::<usize, extern "C" fn(*mut lucet_vmctx, u64) -> u64>(
                func.ptr.as_usize(),
            );
            (func)(vmctx.as_raw(), x) + 1
        }

        #[lucet_hostcall]
        #[no_mangle]
        pub unsafe extern "C" fn add_4_hostcall(
            vmctx: &mut Vmctx,
            x: u64,
            y: u64,
            z: u64,
            w: u64,
        ) -> u64 {
            x + y + z + w
        }

        #[test]
        fn entrypoint_callback() {
            let module =
                test_module_c("entrypoint", "callback.c").expect("module builds and loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let retval = inst
                .run("callback_entrypoint", &[0u64.into()])
                .expect("instance runs")
                .unwrap_returned();
            assert_eq!(u64::from(retval), 3);
        }
    };
}
