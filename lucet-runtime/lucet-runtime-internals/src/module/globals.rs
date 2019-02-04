use crate::error::Error;
use bitflags::bitflags;
use libc::{c_char, int64_t, uint64_t};
use std::ffi::CStr;

/// Specifications from the WebAssembly module about its globals.
///
/// Each module's shared library exports a symbol `lucet_globals_spec` that is first a `GlobalsSpec`
/// struct, followed immediately in memory by the number of `GlobalDescriptor` specified in the
/// spec. These values correspond to the globals of the module, and are ordered by their WebAssembly
/// global index.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct GlobalsSpec {
    pub num_globals: uint64_t,
}

#[repr(C)]
pub struct GlobalsDescriptor {
    flags: GlobalsDescriptorFlags,
    /// Only valid for non-`Import` globals.
    initial_value: int64_t,
    /// Only valid for globals with `ValidName` set.
    name_ptr: *const c_char,
}

bitflags! {
    pub struct GlobalsDescriptorFlags: uint64_t {
        /// Each global is either an internal definition or an import.
        ///
        /// Internal definitions always have an initial value provided in the descriptor. The
        /// initial value of an import is given by the environment (if possible).
        const Import = 1 << 0;

        /// Imports always have a name, and internal definitions sometimes have a name (if they are
        /// marked as "export").
        const ValidName = 1 << 1;
    }
}

/// Read globals from a pointer into a `lucetc`-generated module.
///
/// This function assumes the layout described in the documentation for `GlobalsSpec`, and returns
/// the values of the globals in a vector indexed by their WebAssembly global index.
///
/// For the moment, we don't support import globals, and don't do anything with the names of globals
/// except to provide a better error message when we find an import.
pub unsafe fn read_from_module(spec: *const GlobalsSpec) -> Result<Vec<i64>, Error> {
    // global index is only a u32, so this is a safe cast; the obj format should probably have it as
    // a u32 instead
    let num = spec.as_ref().expect("spec pointer is non-null").num_globals as u32;
    let descriptors_base = spec.offset(1) as *const GlobalsDescriptor;
    let mut globals = vec![0; num as usize];
    for i in 0..num {
        let desc = descriptors_base
            .offset(i as isize)
            .as_ref()
            .expect("descriptor pointer is non-null");
        if desc.flags.contains(GlobalsDescriptorFlags::Import) {
            let name = {
                if !desc.name_ptr.is_null() {
                    CStr::from_ptr(desc.name_ptr)
                } else {
                    CStr::from_bytes_with_nul_unchecked(b"<unknown>\0")
                }
            };
            return Err(Error::Unsupported(format!(
                "import globals are not supported; found import `{}`",
                name.to_string_lossy()
            )));
        } else {
            // non-import globals always have an initial value
            // eprintln!("globals[{}] = {}", i, desc.initial_value);
            globals[i as usize] = desc.initial_value;
        }
    }
    Ok(globals)
}

#[no_mangle]
pub unsafe extern "C" fn lucet_vmctx_get_globals(vmctx: *const ()) -> *const int64_t {
    *(vmctx as *const usize).offset(-1) as *const int64_t
}

#[macro_export]
macro_rules! globals_tests {
    ( $TestRegion:path ) => {
        use $TestRegion as TestRegion;
        use $crate::alloc::Limits;
        use $crate::instance::InstanceInternal;
        use $crate::module::{DlModule, ModuleInternal};
        use $crate::region::Region;

        const INTERNAL_MOD_PATH: &'static str = "lucet-runtime-c/test/build/globals/internal.so";
        const C_IMPORT_MOD_PATH: &'static str = "lucet-runtime-c/test/build/globals/import.so";
        const WAT_IMPORT_MOD_PATH: &'static str = "tests/build/globals_guests/import.so";
        const DEFINITION_SANDBOX_PATH: &'static str = "tests/build/globals_guests/definition.so";

        #[test]
        fn parse_internal() {
            let module = DlModule::load_test(INTERNAL_MOD_PATH).expect("module loads");
            assert_eq!(module.globals()[0], -1);
            assert_eq!(module.globals()[1], 420);
        }

        #[test]
        fn c_reject_import() {
            let module = DlModule::load_test(C_IMPORT_MOD_PATH);
            assert!(module.is_err(), "module load should not succeed");
        }

        #[test]
        fn wat_reject_import() {
            let module = DlModule::load_test(WAT_IMPORT_MOD_PATH);
            assert!(module.is_err(), "module load should not succeed");
        }

        #[test]
        fn read_global0() {
            let module = DlModule::load_test(INTERNAL_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), -1);
        }

        #[test]
        fn read_both_globals() {
            let module = DlModule::load_test(INTERNAL_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), -1);

            let retval = inst.run(b"get_global1", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), 420);
        }

        #[test]
        fn mutate_global0() {
            let module = DlModule::load_test(INTERNAL_MOD_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"set_global0", &[666i64.into()])
                .expect("instance runs");

            let retval = inst.run(b"get_global0", &[]).expect("instance runs");
            assert_eq!(i64::from(retval), 666);
        }

        #[test]
        fn defined_globals() {
            let module = DlModule::load_test(DEFINITION_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let mut inst = region
                .new_instance(Box::new(module))
                .expect("instance can be created");

            inst.run(b"main", &[]).expect("instance runs");

            // Now the globals should be:
            // $x = 3
            // $y = 2
            // $z = 6
            // and heap should be:
            // [0] = 4
            // [4] = 5
            // [8] = 6

            let heap_u32 = unsafe { inst.alloc().heap_u32() };
            assert_eq!(heap_u32[0..=2], [4, 5, 6]);

            inst.run(b"main", &[]).expect("instance runs");

            // now heap should be:
            // [0] = 3
            // [4] = 2
            // [8] = 6

            let heap_u32 = unsafe { inst.alloc().heap_u32() };
            assert_eq!(heap_u32[0..=2], [3, 2, 6]);
        }
    };
}

#[cfg(test)]
mod tests {
    globals_tests!(crate::region::mmap::MmapRegion);
}
