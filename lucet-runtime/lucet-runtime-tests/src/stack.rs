use anyhow::Error;
use lucet_runtime_internals::module::DlModule;
use lucetc::Lucetc;
use std::sync::Arc;
use tempfile::TempDir;

pub fn stack_testcase(num_locals: usize) -> Result<Arc<DlModule>, Error> {
    let native_build = Lucetc::try_from_bytes(generate_test_wat(num_locals, &[], None))?;

    let workdir = TempDir::new().expect("create working directory");

    let so_file = workdir.path().join("out.so");

    native_build.shared_object_file(so_file.clone())?;

    let dlmodule = DlModule::load(so_file)?;

    Ok(dlmodule)
}

// Generate a stack-heavy test wat. This is somewhat modular to support both use in `stack` tests,
// and some slightly more complex tests in `guest_fault`.
//
// `hostcalls` must be a (possibly-empty) list of functions of type `() -> i64`, exported by the
// provided name.
// `recursive_body` must be a wasm body taking `i32` and returning an `i32`. Empty string is
// permissible, but `None` is allowed for caller-clarity.
pub fn generate_test_wat(
    num_i32_locals: usize,
    hostcalls: &[&str],
    recursive_body: Option<&str>,
) -> String {
    // N.B.: because the new backend stack-aligns all local slots to 8 bytes on
    // a 64-bit platform while the old does not, and because we want this test
    // to work for both old and new backends when making specific assertions
    // about stack usage, we use only `i64` locals here. Tests were historically
    // written and tuned using `i32` local counts, so we adjust here while
    // keeping the caller's unit the same.
    let num_locals = num_i32_locals / 2;
    assert!(num_locals > 2);

    let mut module = "(module\n".to_string();
    for hostcall in hostcalls {
        // add an imported hostcall like
        // `(func $foo (import "env" "foo") (result i64))`
        module.push_str(&format!(
            "  (func ${} (import \"env\" \"{}\") (result i64))\n",
            hostcall, hostcall
        ));
    }
    module.push_str("  (func $localpalooza (export \"localpalooza\") (param i64) (result i64)\n");

    // Declare locals:
    module.push_str("(local ");
    for _ in 0..num_locals {
        module.push_str("i64 ");
    }
    module.push_str(")\n");

    // Use each local for the first time:
    for i in 1..num_locals {
        module.push_str(&format!(
            "(set_local {} (i64.add (get_local {}) (i64.xor (get_local {}) (i64.const {}))))\n",
            i,
            i - 1,
            i,
            i
        ));
    }

    // Use each local for a second time, so they get pushed to the stack between uses:
    for i in 2..(num_locals - 1) {
        module.push_str(&format!(
            "(set_local {} (i64.add (get_local {}) (i64.and (get_local {}) (i64.const {}))))\n",
            i,
            i - 1,
            i,
            i
        ));
    }

    // Keep locals alive across a recursive call. Make as many recursive calls as the first
    // argument to the func:
    module.push_str("(if (i32.wrap_i64 (get_local 0))\n");
    module.push_str(&format!(
        "  (then (set_local {} (i64.add (get_local {})\n",
        num_locals - 1,
        num_locals - 2
    ));
    if let Some(body) = recursive_body {
        module.push_str(body);
    }
    module.push_str("      (call $localpalooza (i64.sub (get_local 0) (i64.const 1))))))\n");
    module.push_str(&format!(
        "  (else (set_local {} (i64.add (get_local {}) (get_local {})))))\n",
        num_locals - 1,
        num_locals - 2,
        num_locals - 3
    ));

    module.push_str(&format!("(get_local {})\n", num_locals - 1));
    module.push_str("))\n");
    module
}

#[macro_export]
macro_rules! stack_tests {
    ( $( $region_id:ident => $TestRegion:path ),* ) => {
        $(
            mod $region_id {
                use lucet_runtime::{
                    DlModule, Error, InstanceHandle, Limits, Region, RegionCreate, TrapCode, UntypedRetVal, Val,
                };
                use std::sync::Arc;
                use $TestRegion as TestRegion;
                use $crate::stack::stack_testcase;

                fn run(module: Arc<DlModule>, recursion_depth: i32) -> Result<UntypedRetVal, Error> {
                    let region = <TestRegion as RegionCreate>::create(1, &Limits::default()).expect("region can be created");
                    let mut inst = region
                        .new_instance(module)
                        .expect("instance can be created");

                    let recursion_depth = recursion_depth as i64;
                    inst.run("localpalooza", &[recursion_depth.into()])
                        .and_then(|rr| rr.returned())
                }

                fn expect_ok(module: Arc<DlModule>, recursion_depth: i32) {
                    assert!(run(module, recursion_depth).is_ok());
                }

                fn expect_stack_overflow(module: Arc<DlModule>, recursion_depth: i32, probestack: bool) {
                    match run(module, recursion_depth) {
                        Err(Error::RuntimeFault(details)) => {
                            // We should get a nonfatal trap due to the stack overflow.
                            assert_eq!(details.fatal, false);
                            assert_eq!(details.trapcode, Some(TrapCode::StackOverflow));
                            if probestack {
                                // Make sure we overflowed in the stack probe as expected
                                //
                                // TODO: this no longer differentiates between different stack overflow
                                // sites after moving the stack probe into lucetc; figure out a way to
                                // provide that information or just wait till we can do the stack probe as a
                                // global symbol again
                                let addr_details =
                                    details.rip_addr_details.expect("can look up addr details");
                                assert!(addr_details.in_module_code);
                            }
                        }
                        res => panic!("unexpected result: {:?}", res),
                    }
                }

                #[test]
                fn ensure_linked() {
                    lucet_runtime::lucet_internal_ensure_linked();
                }

                #[test]
                fn expect_ok_locals6_1() {
                    expect_ok(stack_testcase(6).expect("generate stack_testcase 6"), 1);
                }

                // The test with 64 locals should cause a stack overflow on the 481st recursion
                // with the old backend. The trap table knows about all of the instructions in the function
                // that manipulate the stack, so the catch mechanism for this is the usual one.

                #[test]
                fn expect_ok_locals64_1() {
                    expect_ok(stack_testcase(64).expect("generate stack_testcase 64"), 1);
                }

                #[test]
                fn expect_ok_locals64_2() {
                    expect_ok(stack_testcase(64).expect("generate stack_testcase 64"), 2);
                }

                #[test]
                fn expect_ok_locals64_480() {
                    // We use 64 local variables, but cranelift optimizes many of them into registers
                    // rather than stack space. Four of those are callee-saved, for n > 18 local
                    // variables, we actually use n + 4 stack spaces. So we use 64 spaces at 64 - 4 = 60
                    // local variables.
                    expect_ok(
                        stack_testcase(64 - 4).expect("generate stack_testcase 64"),
                        480,
                    );
                }

                #[test]
                fn expect_stack_overflow_locals64_481() {
                    expect_stack_overflow(
                        // Same note as `expect_ok_locals64_481`
                        stack_testcase(64 - 4).expect("generate stack_testcase 64"),
                        1000,
                        true,
                    );
                }

                // 1050 locals is about 1 page (4k) on the stack - just enough for Cranelift to use probestack to grow
                // the stack. The 31st recursion should cause a stack overflow.
                //
                // With the new backend, all local stackslots are 8-byte aligned, so the stack
                // usage is roughly twice as high here. So 31 recursions will cause an overflow in
                // all cases, but we don't assert that 30 is OK; rather, we test at 14.

                #[test]
                fn expect_ok_locals_1page_1() {
                    expect_ok(
                        stack_testcase(1050).expect("generate stack_testcase 1050"),
                        1,
                    );
                }

                #[test]
                fn expect_ok_locals_1page_2() {
                    expect_ok(
                        stack_testcase(1050).expect("generate stack_testcase 1050"),
                        2,
                    );
                }

                #[test]
                fn expect_ok_locals_1page_14() {
                    expect_ok(
                        stack_testcase(1050).expect("generate stack_testcase 1050"),
                        14,
                    );
                }

                #[test]
                fn expect_stack_overflow_locals_1page_31() {
                    expect_stack_overflow(
                        stack_testcase(1050).expect("generate stack_testcase 1050"),
                        31,
                        true,
                    );
                }

                // This test has 5000 locals - over 4 pages worth. Cranelift will use probestack here as well. The
                // 6th recursion should cause a stack overflow.

                #[test]
                fn expect_ok_locals_multipage_1() {
                    expect_ok(
                        stack_testcase(5000).expect("generate stack_testcase 5000"),
                        1,
                    );
                }

                #[test]
                fn expect_ok_locals_multipage_2() {
                    expect_ok(
                        stack_testcase(5000).expect("generate stack_testcase 5000"),
                        2,
                    );
                }

                #[test]
                fn expect_ok_locals_multipage_5() {
                    expect_ok(
                        stack_testcase(5000).expect("generate stack_testcase 5000"),
                        5,
                    );
                }

                #[test]
                fn expect_stack_overflow_locals_multipage_6() {
                    expect_stack_overflow(
                        stack_testcase(5000).expect("generate stack_testcase 5000"),
                        6,
                        true,
                    );
                }
            }
        )*
    };
}
