use std::path::PathBuf;

fn run_core_spec_test(name: &str) {
    let file = PathBuf::from(&format!("spec/test/core/{}.wast", name));
    assert!(file.exists());
    let run = lucet_spectest::run_spec_test(&file).unwrap();
    run.report(); // Print to stdout
    if run.failed().len() > 0 {
        panic!("{} had {} failures", name, run.failed().len());
    }
}

macro_rules! core_spec_test {
    ($base_name:ident) => {
        #[test]
        pub fn $base_name() {
            run_core_spec_test(stringify!($base_name));
        }
    };
    // Some spec tests have filenames that are not valid rust identifiers. We make a valid
    // identifier for the base_name by hand, and provide the filename as a string.
    ($base_name:ident, $file_name:expr) => {
        #[test]
        pub fn $base_name() {
            run_core_spec_test($file_name);
        }
    };
}

core_spec_test!(address); // PASS
core_spec_test!(align); // PASS
core_spec_test!(binary); // PASS
core_spec_test!(block); // PASS
core_spec_test!(break_drop, "break-drop"); // PASS
core_spec_test!(br_if); // PASS
core_spec_test!(br_table); // PASS
core_spec_test!(br); // PASS
core_spec_test!(call_indirect); // PASS
core_spec_test!(call); // PASS
core_spec_test!(comments); // PASS
core_spec_test!(const_, "const"); // PASS
core_spec_test!(conversions); // PASS
core_spec_test!(custom); // PASS
core_spec_test!(data); // FAIL: non-const data init expr
core_spec_test!(elem); // FAIL: non-const data init expr
core_spec_test!(endianness); // PASS
core_spec_test!(exports); // PASS (3 skipped)
core_spec_test!(f32_bitwise); // PASS
core_spec_test!(f32_cmp); // PASS
core_spec_test!(f32_, "f32"); // PASS
core_spec_test!(f64_bitwise); // PASS
core_spec_test!(f64_cmp); // PASS
core_spec_test!(f64_, "f64"); // PASS
core_spec_test!(fac); // PASS
core_spec_test!(float_exprs); // PASS
core_spec_test!(float_literals); // PASS
core_spec_test!(float_memory); // PASS
core_spec_test!(float_misc); // PASS
core_spec_test!(forward); // PASS
core_spec_test!(func_ptrs); // PASS
core_spec_test!(func); // PASS
core_spec_test!(get_local); // PASS
core_spec_test!(globals); // FAIL: exports mutable globals, which wabt does not support
core_spec_test!(i32_, "i32"); // PASS
core_spec_test!(i64_, "i64"); // PASS
core_spec_test!(if_, "if"); // PASS
                            // currently stops at 'creation of elements for undeclared table!', which is actually due to an elem section populating an imported table.
                            // past that, lots of unexpected success or incorrect results, some BadSignature faults, some "symbol not found" errors indicating test harness isnt correct.
core_spec_test!(imports); // FAIL: see above comment
core_spec_test!(inline_module, "inline-module"); // PASS
core_spec_test!(int_exprs); // PASS
core_spec_test!(int_literals); // PASS
core_spec_test!(labels); // PASS
core_spec_test!(left_to_right, "left-to-right"); // PASS
core_spec_test!(linking); // FAIL: exports mutable globals
core_spec_test!(loop_, "loop"); // PASS
core_spec_test!(memory_grow); // PASS
core_spec_test!(memory_redundancy); // PASS
core_spec_test!(memory_trap); // PASS
core_spec_test!(memory); // PASS
                         // too noisy to keep enabled:
                         // core_spec_test!(names); // FAIL hundreds of errors because we dont support unicode names yet.
core_spec_test!(nop); // PASS
core_spec_test!(return_, "return"); // PASS
core_spec_test!(select); // PASS
core_spec_test!(set_local); // PASS
core_spec_test!(skip_stack_guard_page, "skip-stack-guard-page"); // PASS but takes over 1 minute in cranelift building function-with-many-locals
core_spec_test!(stack); // PASS
core_spec_test!(start); // PASS
core_spec_test!(store_retval); // PASS
core_spec_test!(switch); // PASS
core_spec_test!(tee_local); // PASS
core_spec_test!(token); // PASS
core_spec_test!(traps); // PASS
core_spec_test!(typecheck); // PASS
core_spec_test!(type_, "type"); // PASS
core_spec_test!(unreachable); // PASS
core_spec_test!(unreached_invaild, "unreached-invalid"); // PASS
core_spec_test!(unwind); // PASS
core_spec_test!(utf8_custom_section_id, "utf8-custom-section-id"); // PASS
core_spec_test!(utf8_import_field, "utf8-import-field"); // PASS
core_spec_test!(utf8_import_module, "utf8-import-module"); // PASS
core_spec_test!(utf8_invalid_encoding, "utf8-invalid-encoding"); // PASS
