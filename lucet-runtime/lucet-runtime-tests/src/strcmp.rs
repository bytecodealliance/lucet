#[macro_export]
macro_rules! strcmp_tests {
    ( $TestRegion:path ) => {
        use libc::{c_char, c_int, c_void, strcmp, uint64_t};
        use lucet_runtime::vmctx::lucet_vmctx;
        use lucet_runtime::{DlModule, Error, Limits, Region, Val, WASM_PAGE_SIZE};
        use std::ffi::CString;
        use std::sync::Arc;
        use $TestRegion as TestRegion;
        use $crate::helpers::DlModuleExt;

        const FAULT_MOD_PATH: &'static str = "tests/build/strcmp_guests/fault_guest.so";

        #[no_mangle]
        unsafe extern "C" fn hostcall_host_fault(_vmctx: *const lucet_vmctx) {
            let oob = (-1isize) as *mut c_char;
            *oob = 'x' as c_char;
        }

        fn strcmp_compare(s1: &str, s2: &str) {
            let s1 = CString::new(s1)
                .expect("s1 is a valid CString")
                .into_bytes_with_nul();
            let s2 = CString::new(s2)
                .expect("s2 is a valid CString")
                .into_bytes_with_nul();

            let res_size = std::mem::size_of::<uint64_t>();
            assert!(res_size + s1.len() + s2.len() < WASM_PAGE_SIZE as usize);

            let module = DlModule::load_test(FAULT_MOD_PATH).expect("module loads");
            let region = TestRegion::create(10, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            let newpage_start = inst.grow_memory(1).expect("grow_memory succeeds");
            let heap = inst.heap_mut();

            let res_ptr = (newpage_start * WASM_PAGE_SIZE) as usize;
            let s1_ptr = res_ptr + res_size;
            let s2_ptr = s1_ptr + s1.len();
            heap[s1_ptr..s2_ptr].copy_from_slice(&s1);
            heap[s2_ptr..s2_ptr + s2.len()].copy_from_slice(&s2);

            let res = c_int::from(
                inst.run(
                    b"run_strcmp",
                    &[
                        Val::GuestPtr(s1_ptr as u32),
                        Val::GuestPtr(s2_ptr as u32),
                        Val::GuestPtr(res_ptr as u32),
                    ],
                )
                .expect("instance runs"),
            );

            let host_strcmp_res =
                unsafe { strcmp(s1.as_ptr() as *const c_char, s2.as_ptr() as *const c_char) };
            assert_eq!(res, host_strcmp_res);
        }

        #[test]
        fn abc_abc() {
            strcmp_compare("abc", "abc");
        }

        #[test]
        fn def_abc() {
            strcmp_compare("def", "abc");
        }

        #[test]
        fn abcd_abc() {
            strcmp_compare("abcd", "abc");
        }

        #[test]
        fn abc_abcd() {
            strcmp_compare("abc", "abcd");
        }

        #[test]
        fn wasm_fault_test() {
            let module = DlModule::load_test(FAULT_MOD_PATH).expect("module loads");
            let region = TestRegion::create(10, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(module)
                .expect("instance can be created");

            match inst.run(b"wasm_fault", &[]) {
                Err(Error::RuntimeFault { .. }) => (),
                res => panic!("unexpected result: {:?}", res),
            }
        }
    };
}
