use lucet_runtime_tests::timeout_tests;

timeout_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
