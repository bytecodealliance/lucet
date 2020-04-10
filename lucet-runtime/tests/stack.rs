use lucet_runtime_tests::stack_tests;

cfg_if::cfg_if! {
    if #[cfg(all(target_os = "linux", feature = "uffd"))] {
        stack_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        stack_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
