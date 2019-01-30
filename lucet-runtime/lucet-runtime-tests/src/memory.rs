#[macro_export]
macro_rules! memory_tests {
    ( $TestRegion:path ) => {
        use lazy_static::lazy_static;
        use lucet_libc::LucetLibc;
        use lucet_runtime::instance::State;
        use lucet_runtime::{DlModule, Limits, Region};
        use std::sync::Mutex;
        use $TestRegion as TestRegion;
        use $crate::helpers::DlModuleExt;

        const CURRENT_MEMORY_SANDBOX_PATH: &'static str =
            "tests/build/memory_guests/current_memory.so";
        const GROW_MEMORY_SANDBOX_PATH: &'static str = "tests/build/memory_guests/grow_memory.so";
        const MUSL_ALLOC_SANDBOX_PATH: &'static str = "tests/build/memory_guests/musl_alloc.so";

        #[test]
        fn current_memory_hostcall() {
            let module = DlModule::load_test(CURRENT_MEMORY_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            match &inst.state {
                State::Ready { retval } => {
                    // WebAssembly module requires 4 pages of memory in import
                    assert_eq!(u32::from(retval), 4);
                }
                st => panic!("unexpected state: {}", st),
            }
        }

        #[test]
        fn grow_memory_hostcall() {
            let module = DlModule::load_test(GROW_MEMORY_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");
            assert!(inst.is_ready());

            let heap = inst.heap_u32();
            // guest puts the result of the grow_memory(1) call in heap[0]; based on the current settings,
            // growing by 1 returns prev size 4
            assert_eq!(heap[0], 4);
            // guest then puts the result of the current memory call in heap[4] (indexed by bytes)
            assert_eq!(heap[1], 5);
        }

        #[test]
        fn musl_alloc() {
            lazy_static! {
                static ref OUTPUT_STRING: Mutex<String> = Mutex::new(String::new());
            }

            macro_rules! assert_output_eq {
                ( $s:expr ) => {
                    assert_eq!($s, &*OUTPUT_STRING.lock().unwrap())
                };
            }

            fn reset_output() {
                *OUTPUT_STRING.lock().unwrap() = String::with_capacity(1024);
            }

            extern "C" fn debug_handler(
                _libc: *mut lucet_libc::lucet_libc,
                fd: libc::int32_t,
                buf: *const libc::c_char,
                len: libc::size_t,
            ) {
                assert_eq!(fd, 1);
                let msg = unsafe { std::slice::from_raw_parts(buf as *const u8, len) };
                OUTPUT_STRING
                    .lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(msg));
            }

            let module = DlModule::load_test(MUSL_ALLOC_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");

            reset_output();

            let mut libc = Box::new(LucetLibc::new());
            libc.set_stdio_handler(debug_handler);

            let mut inst = region
                .new_instance_with_ctx(Box::new(module), Box::into_raw(libc) as *mut libc::c_void)
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");
            assert!(inst.is_ready());

            assert_output_eq!("this is a string located in the heap: hello from musl_alloc.c!\n\n");
        }
    };
}
