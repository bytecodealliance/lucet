use lucet_runtime_tests::start_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        start_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        start_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
