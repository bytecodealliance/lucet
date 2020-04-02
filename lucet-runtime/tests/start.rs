use lucet_runtime_tests::start_tests;

start_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
