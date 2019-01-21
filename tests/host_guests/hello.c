#include <stddef.h>

extern void hostcall_test_func_hello(const char *hello_ptr, size_t hello_len);

__attribute__((visibility("default"))) int main(void)
{
    char hello[] = "hello world";
    hostcall_test_func_hello(hello, sizeof(hello));
    return 0;
}
