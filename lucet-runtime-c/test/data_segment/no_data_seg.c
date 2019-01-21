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

// liblucet expects the WASM .so it loads to supply WASM data segment
// iniitialization info via the symbols defined below. liblucet uses this info
// to copy initial data into linear memory when a module is instantiated.
//
// Presently liblucet is using a format implicitly defined in cton-lucet, which is
// mimicked below.

const uint32_t wasm_data_segments_len = 4444;
void           guest_main(void *ctx) {}

int main(void)
{
    return 0;
}
