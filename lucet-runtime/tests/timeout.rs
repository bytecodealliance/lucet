use lucet_runtime_tests::timeout_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        timeout_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        timeout_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
