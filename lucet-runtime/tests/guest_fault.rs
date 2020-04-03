use lucet_runtime_tests::guest_fault_tests;

cfg_if::cfg_if! {
    if #[cfg(feature = "uffd")] {
        guest_fault_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        guest_fault_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
