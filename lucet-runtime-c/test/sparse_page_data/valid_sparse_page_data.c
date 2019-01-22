#include <inttypes.h>

#include "../../src/lucet_alloc_private.h"
#include "../helpers.h"

DEFINE_DEFAULT_DATA_SEGMENTS;

struct lucet_alloc_heap_spec lucet_heap_spec = {
    .initial_size = 64 * 1024,
    .max_size     = 64 * 1024,
    .guard_size   = 4 * 1024 * 1024,
};
struct lucet_globals_spec lucet_globals_spec = {
    .num_globals = 0,
};

static char first_page[4096] = "hello from valid_sparse_page_data.c!";
static char third_page[4096] = "hello again from valid_sparse_page_data.c!";

// liblucet expects the WASM .so it loads to supply sparse page data via the
// `guest_sparse_page_data` symbol. liblucet uses this info to copy initial data into linear memory
//
// Presently liblucet is using a format implicitly defined in cton-lucet, which is mimicked below.
const uint64_t guest_sparse_page_data[] = {
    3, // num_pages
    (uint64_t) &first_page,
    0, // NULL
    (uint64_t) &third_page,
};

void guest_main(void *ctx) {}

int main(void)
{
    return 0;
}
