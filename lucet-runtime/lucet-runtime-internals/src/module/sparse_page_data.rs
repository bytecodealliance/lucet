

#[macro_export]
macro_rules! sparse_page_data_tests {
    ( $TestRegion:path ) => {
        /* TODO fix all this!!!
        use $TestRegion as TestRegion;
        use $crate::alloc::{host_page_size, Limits};
        use $crate::instance::InstanceInternal;
        use $crate::module::DlModule;
        use $crate::region::Region;


        // XXX use an OwnedModuleData to create the sparse data, and instantiate the mock module to
        // make sure it follows the spec.

        #[test]
        fn instantiate_valid_sparse_data() {
            let module = DlModule::load_test(VALID_SANDBOX_PATH).expect("module loads");
            let region = TestRegion::create(1, &Limits::default()).expect("region can be created");
            let inst = region
                .new_instance(module)
                .expect("instance can be created");

            // The test data initializers result in two strings getting copied into linear memory; see
            // `lucet-runtime-c/test/data_segment/valid_data_seg.c` for details
            let heap = unsafe { inst.alloc().heap() };
            assert_eq!(&heap[0..FIRST_MESSAGE.len()], FIRST_MESSAGE.as_ref());
            let second_message_start = 2 * host_page_size();
            assert_eq!(
                &heap[second_message_start..second_message_start + SECOND_MESSAGE.len()],
                SECOND_MESSAGE.as_ref()
            );
        }
        */
    };
}

#[cfg(test)]
mod tests {
    sparse_page_data_tests!(crate::region::mmap::MmapRegion);
}
