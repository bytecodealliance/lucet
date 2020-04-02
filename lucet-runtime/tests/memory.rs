use lucet_runtime_tests::memory_tests;

memory_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
