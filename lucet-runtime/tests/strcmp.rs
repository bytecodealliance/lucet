use lucet_runtime_tests::strcmp_tests;

cfg_if::cfg_if! {
    if #[cfg(feature = "uffd")] {
        strcmp_tests!(
            mmap => lucet_runtime::MmapRegion,
            uffd => lucet_runtime::UffdRegion
        );
    } else {
        strcmp_tests!(mmap => lucet_runtime::MmapRegion);
    }
}
