#ifndef WRAPPER_H
#define WRAPPER_H

#ifdef LUCET_TEST_EXTRA_INCLUDE
#include LUCET_TEST_EXTRA_INCLUDE
#endif // LUCET_TEST_EXTRA_INCLUDE

#define ASSERT_OK(E) assert((E) == lucet_error_ok)

#ifndef lucet_test_region
#define lucet_test_region lucet_mmap_region
#endif // lucet_test_region

#ifndef lucet_test_region_create
#define lucet_test_region_create lucet_mmap_region_create
#endif // lucet_test_region_create

#ifndef lucet_test_region_new_instance
#define lucet_test_region_new_instance lucet_mmap_region_new_instance
#endif // lucet_test_region_new_instance

#ifndef lucet_test_region_new_instance_with_ctx
#define lucet_test_region_new_instance_with_ctx lucet_mmap_region_new_instance_with_ctx
#endif // lucet_test_region_new_instance_with_ctx

#ifndef lucet_test_region_release
#define lucet_test_region_release lucet_mmap_region_release
#endif // lucet_test_region_release

#endif // WRAPPER_H
