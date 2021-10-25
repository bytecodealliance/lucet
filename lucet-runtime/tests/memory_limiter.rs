use lucet_runtime_tests::memory_limiter_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        memory_limiter_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        memory_limiter_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
