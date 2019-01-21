#include <inttypes.h>

#include "../../src/lucet_alloc_private.h"
#include "../helpers.h"

DEFINE_DEFAULT_HEAP_SPEC;
DEFINE_DEFAULT_GLOBAL_SPEC;
DEFINE_DEFAULT_DATA_SEGMENTS;

void guest_func_main(void *ctx)
{
    return;
}
