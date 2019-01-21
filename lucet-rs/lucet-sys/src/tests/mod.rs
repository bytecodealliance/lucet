use super::*;
use std::ffi::CString;
use std::path::PathBuf;

#[test]
fn create_pool() {
    let limits = lucet_alloc_limits {
        heap_memory_size: 64 * 1024,
        heap_address_space_size: 8 * 1024 * 1024,
        stack_size: 128 * 1024,
        globals_size: 4 * 1024,
    };
    let pool = unsafe { lucet_pool_create(1000, &limits) };
    assert!(!pool.is_null());
}

#[test]
fn run() {
    let limits = lucet_alloc_limits {
        heap_memory_size: 64 * 1024,
        heap_address_space_size: 8 * 1024 * 1024,
        stack_size: 128 * 1024,
        globals_size: 4 * 1024,
    };
    let pool = unsafe { lucet_pool_create(1000, &limits) };
    assert!(!pool.is_null());

    let mut module_code = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    module_code.push("..");
    module_code.push("..");
    module_code.push("lucet-runtime-c");
    module_code.push("test");
    module_code.push("build");
    module_code.push("globals");
    module_code.push("internal.so");

    let module_code = std::fs::canonicalize(module_code).expect("absolute path");
    assert!(module_code.exists(), format!("test module is part of liblucet test suite build - run `make test` in lucet-runtime-c to make sure it exists at {}", module_code.display()));

    let module_code = CString::new(format!("{}", module_code.display())).expect("valid c string");
    let module = unsafe { lucet_module_load(module_code.as_ptr()) };
    assert!(!module.is_null());
}
