use lucet_runtime_tests::{guest_fault_common_defs, guest_fault_tests};

guest_fault_common_defs!();

guest_fault_tests!(lucet_runtime::MmapRegion);
