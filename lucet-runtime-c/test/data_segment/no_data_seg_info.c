#include <inttypes.h>

#include "../../src/lucet_alloc_private.h"

struct lucet_alloc_heap_spec lucet_heap_spec = {
    .initial_size = 64 * 1024,
    .max_size     = 64 * 1024,
    .guard_size   = 4 * 1024 * 1024,
};
struct lucet_globals_spec lucet_globals_spec = {
    .num_globals = 0,
};

void guest_main(void *ctx) {}

int main(void)
{
    return 0;
}
