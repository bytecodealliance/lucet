use lucet_runtime_tests::host_tests;

host_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
