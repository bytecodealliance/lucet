#[macro_export]
macro_rules! strcmp_tests {
    ( $TestRegion:path ) => {
        use libc::{c_char, c_int, c_void, strcmp};
        use lucet_runtime::vmctx::{lucet_vmctx, Vmctx};
        use lucet_runtime::{lucet_hostcall, Error, Limits, Region, Val, WASM_PAGE_SIZE};
        use std::ffi::CString;
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::build::test_module_c;

        #[lucet_hostcall]
        #[no_mangle]
        pub fn hostcall_host_fault(_vmctx: &mut Vmctx) {
            let oob = (-1isize) as *mut c_char;
            unsafe {
                *oob = 'x' as c_char;
            }
        }

        fn strcmp_compare(s1: &str, s2: &str) {
            let s1 = CString::new(s1)
                .expect("s1 is a valid CString")
                .into_bytes_with_nul();
            let s2 = CString::new(s2)
                .expect("s2 is a valid CString")
                .into_bytes_with_nul();

            assert!(s1.len() + s2.len() < WASM_PAGE_SIZE as usize);

            let module = test_module_c("strcmp", "guest.c").expect("compile module");
            let region = TestRegion::create(10, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let newpage_start = inst.grow_memory(1).expect("grow_memory succeeds");
            let heap = inst.heap_mut();

            let s1_ptr = (newpage_start * WASM_PAGE_SIZE) as usize;
            let s2_ptr = s1_ptr + s1.len();
            heap[s1_ptr..s2_ptr].copy_from_slice(&s1);
            heap[s2_ptr..s2_ptr + s2.len()].copy_from_slice(&s2);

            let res = c_int::from(
                inst.run(
                    "run_strcmp",
                    &[Val::GuestPtr(s1_ptr as u32), Val::GuestPtr(s2_ptr as u32)],
                )
                .expect("instance runs")
                .unwrap_returned(),
            );

            let host_strcmp_res =
                unsafe { strcmp(s1.as_ptr() as *const c_char, s2.as_ptr() as *const c_char) };
            assert_eq!(res, host_strcmp_res);
        }

        #[test]
        fn strcmp_abc_abc() {
            strcmp_compare("abc", "abc");
        }

        #[test]
        fn strcmp_def_abc() {
            strcmp_compare("def", "abc");
        }

        #[test]
        fn strcmp_abcd_abc() {
            strcmp_compare("abcd", "abc");
        }

        #[test]
        fn strcmp_abc_abcd() {
            strcmp_compare("abc", "abcd");
        }

        #[test]
        fn strcmp_fault_test() {
            let module = test_module_c("strcmp", "guest.c").expect("compile module");
            let region = TestRegion::create(10, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run("wasm_fault", &[]) {
                Err(Error::RuntimeFault { .. }) => (),
                res => panic!("unexpected result: {:?}", res),
            }
        }
    };
}
