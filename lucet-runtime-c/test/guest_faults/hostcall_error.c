#include <inttypes.h>

#include "../../src/lucet_alloc_private.h"
#include "../helpers.h"

DEFINE_DEFAULT_HEAP_SPEC;
DEFINE_DEFAULT_GLOBAL_SPEC;
DEFINE_DEFAULT_DATA_SEGMENTS;

extern void hostcall_test(void);

void guest_func_main(void *ctx)
{
    hostcall_test();
    // hostcall_test should never return, so if we get an illegal instruction
    // fault we know something is wrong.
    asm("ud2");
}

int guest_func_onetwothree(void *ctx)
{
    return 123;
}
