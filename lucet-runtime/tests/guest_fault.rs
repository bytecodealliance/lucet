use lucet_runtime_tests::guest_fault_tests;

guest_fault_tests!(
    mmap => lucet_runtime::MmapRegion,
    uffd => lucet_runtime::UffdRegion
);
