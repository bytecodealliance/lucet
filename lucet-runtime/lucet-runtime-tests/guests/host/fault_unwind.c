#include <assert.h>
#include <stdint.h>

extern void bad_access_unwind(void (*)(void));
extern void stack_overflow_unwind(void (*)(void));

void do_bad_access(void)
{
    *(uint64_t *) 0xFFFFFFFF = 420;
    return;
}

void bad_access(void)
{
    return bad_access_unwind(do_bad_access);
}

void do_stack_overflow(void) {
    do_stack_overflow();
}

void stack_overflow(void)
{
    return stack_overflow_unwind(do_stack_overflow);
}
