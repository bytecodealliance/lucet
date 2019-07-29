#include <stdint.h>

extern uint64_t lucet_runtime_test_hostcall_yield_resume(uint64_t n);

void f()
{
    lucet_runtime_test_hostcall_yield_resume(5);
}
