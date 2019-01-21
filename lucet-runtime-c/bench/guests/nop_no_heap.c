#include "../../src/lucet_alloc_private.h"
#include "lucet_vmctx.h"
#include <inttypes.h>

struct lucet_alloc_heap_spec lucet_heap_spec = {
    .guard_size     = 4 * 1024 * 1024,
    .reserved_size  = 4 * 1024 * 1024,
    .initial_size   = 64 * 1024,
    .max_size       = 64 * 1024,
    .max_size_valid = 1,
};
struct lucet_globals_spec lucet_globals_spec = {
    .num_globals = 0,
};

__attribute__((visibility("default"))) const char wasm_data_segments[] = "";

__attribute__((visibility("default"))) const uint32_t wasm_data_segments_len = 0;

int main()
{
    return 0;
}
