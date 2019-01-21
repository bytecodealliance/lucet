
#include <stdint.h>

extern void b(uint32_t arg);

void a(uint32_t arg)
{
    b(arg + 1);
}

extern void hostcall_fault(void);

void call_hostcall_fault(void)
{
    hostcall_fault();
}
