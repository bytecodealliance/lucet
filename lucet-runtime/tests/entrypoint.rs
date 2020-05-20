use lucet_runtime_tests::entrypoint_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        entrypoint_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        entrypoint_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
