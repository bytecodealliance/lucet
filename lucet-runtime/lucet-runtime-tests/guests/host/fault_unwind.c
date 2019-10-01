#include <assert.h>
#include <stdint.h>

extern void fault_unwind(void (*)(void));

void callback(void)
{
    *(uint64_t *) 0xFFFFFFFF = 420;
    return;
}

void entrypoint(void)
{
    return fault_unwind(callback);
}
