#[cfg(test)]
mod tests {
    use crate::alloc::{host_page_size, Limits};
    use crate::module::{DlModule, Module};
    use crate::region::mmap::MmapRegion;
    use crate::region::Region;

    const VALID_SANDBOX_PATH: &'static str =
        "lucet-runtime-c/test/build/sparse_page_data/valid_sparse_page_data.so";

    const FIRST_MESSAGE: &'static [u8] = b"hello from valid_sparse_page_data.c!";
    const SECOND_MESSAGE: &'static [u8] = b"hello again from valid_sparse_page_data.c!";

    #[test]
    fn valid_sparse_page_data() {
        let module = DlModule::load_test(VALID_SANDBOX_PATH).expect("module loads");

        let sparse_page_data = module.sparse_page_data().expect("can get sparse page data");

        assert_eq!(sparse_page_data.len(), 3);

        let mut first_page_expected: Vec<u8> = FIRST_MESSAGE.to_vec();
        first_page_expected.resize(host_page_size(), 0);
        let mut third_page_expected: Vec<u8> = SECOND_MESSAGE.to_vec();
        third_page_expected.resize(host_page_size(), 0);

        let first_page: &[u8] = unsafe {
            std::slice::from_raw_parts(sparse_page_data[0] as *const u8, host_page_size())
        };
        assert_eq!(first_page, first_page_expected.as_slice());

        assert!(sparse_page_data[1].is_null());

        let third_page: &[u8] = unsafe {
            std::slice::from_raw_parts(sparse_page_data[2] as *const u8, host_page_size())
        };
        assert_eq!(third_page, third_page_expected.as_slice());
    }

    #[test]
    fn instantiate_valid_sparse_data() {
        let module = DlModule::load_test(VALID_SANDBOX_PATH).expect("module loads");
        let region = MmapRegion::create(1, &Limits::default()).expect("region can be created");
        let inst = region
            .new_instance(Box::new(module))
            .expect("instance can be created");

        // The test data initializers result in two strings getting copied into linear memory; see
        // `lucet-runtime-c/test/data_segment/valid_data_seg.c` for details
        let heap = unsafe { inst.alloc.heap() };
        assert_eq!(&heap[0..FIRST_MESSAGE.len()], FIRST_MESSAGE.as_ref());
        let second_message_start = 2 * host_page_size();
        assert_eq!(
            &heap[second_message_start..second_message_start + SECOND_MESSAGE.len()],
            SECOND_MESSAGE.as_ref()
        );
    }
}
