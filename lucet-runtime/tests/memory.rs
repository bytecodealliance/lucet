use lucet_runtime_tests::memory_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        memory_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        memory_tests!(mmap => lucet_runtime::MmapRegion);
    }
}

#[cfg(all(target_os = "linux", feature = "uffd"))]
mod uffd_specific {
    use libc::{c_void, mincore};
    use lucet_runtime::{Limits, Region};
    use lucet_runtime::{UffdRegion, WasmPageSizedUffdStrategy};
    use lucet_runtime_tests::build::test_module_wasm;

    #[test]
    fn ensure_linked() {
        lucet_runtime::lucet_internal_ensure_linked();
    }
    #[test]
    fn lazy_memory() {
        let module = test_module_wasm("memory", "uffd_memory.wat")
            .expect("compile and load uffd_memory.wasm");
        let region = UffdRegion::create(1, &Limits::default(), WasmPageSizedUffdStrategy {})
            .expect("region can be created");
        let mut inst = region
            .new_instance(module)
            .expect("instance can be created");

        inst.run("main", &[]).expect("instance runs");

        let heap = inst.heap_u32();
        // guest puts 1 at the start of the second page
        assert_eq!(heap[16384], 1);
        // guest then puts 2 at the start of the fourth page
        assert_eq!(heap[49152], 2);

        const HEAP_LEN: usize = 4 * 65536; // 4 WebAssembly Pages
        const VEC_LEN: usize = HEAP_LEN / 4096; // 1 u8 per host page

        let mut result_vec: [u8; VEC_LEN] = [0; VEC_LEN];

        // We use `mincore` here to determine which pages of the heap actually
        // exist in memory, and which are just empty virtual pages.
        let result = unsafe {
            mincore(
                inst.heap_mut().as_mut_ptr() as *mut c_void,
                HEAP_LEN,
                result_vec.as_mut_ptr(),
            )
        };
        assert_eq!(result, 0);

        // As noted above, the instance wrote to the second and fourth wasm
        // pages. So, host pages 16-31 and 48-63 should be mapped to memory
        // and nothing else should be.
        assert_eq!(&result_vec[0..16], &[0; 16]);
        assert_eq!(&result_vec[16..32], &[1; 16]);
        assert_eq!(&result_vec[32..48], &[0; 16]);
        assert_eq!(&result_vec[48..64], &[1; 16]);
    }
}
