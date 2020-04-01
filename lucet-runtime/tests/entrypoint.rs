use lucet_runtime_tests::entrypoint_tests;

entrypoint_tests!(
    lucet_runtime::MmapRegion => mmap,
    lucet_runtime::UffdRegion => uffd
);
