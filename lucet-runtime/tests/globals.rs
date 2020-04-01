use lucet_runtime_tests::globals_tests;

globals_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
