#ifndef TEST_HELPERS_H
#define TEST_HELPERS_H

#include <string.h>

const char *guest_module_path(const char *);

#define ASSERT_OK(E) ASSERT_ENUM_EQ(lucet_error_ok, (E), lucet_error_name)

#endif // TEST_HELPERS_H
