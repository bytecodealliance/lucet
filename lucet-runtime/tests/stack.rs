use lucet_runtime_tests::stack_tests;

stack_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
