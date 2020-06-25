use lucet_runtime_tests::async_hostcall_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        async_hostcall_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        async_hostcall_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
