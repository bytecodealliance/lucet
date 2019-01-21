#include <stddef.h>

extern void hostcall_test_func_hostcall_error(void);

__attribute__((visibility("default"))) int main(void)
{
    hostcall_test_func_hostcall_error();
    return 0;
}
