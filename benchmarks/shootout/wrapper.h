#ifndef WRAPPER_H
#define WRAPPER_H

#ifdef LUCET_TEST_EXTRA_INCLUDE
#include LUCET_TEST_EXTRA_INCLUDE
#endif // LUCET_TEST_EXTRA_INCLUDE

#define ASSERT_OK(E) assert((E) == lucet_error_ok)

#ifndef lucet_test_region_create
#define lucet_test_region_create lucet_mmap_region_create
#endif // lucet_test_region_create

#endif // WRAPPER_H
