use lucet_runtime_tests::entrypoint_tests;

entrypoint_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
