#include <stddef.h>

extern void hostcall_test_func_hostcall_error_unwind(void);

int main(void)
{
    hostcall_test_func_hostcall_error_unwind();
    return 0;
}
