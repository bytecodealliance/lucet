use lucet_runtime_tests::strcmp_tests;

strcmp_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
