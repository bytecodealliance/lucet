#include <inttypes.h>
#include <limits.h>

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
const char wasm_data_segments[] =
    "\x00\x00\x00\x00" // 0: memdix
    "\x00\x00\x00\x00" // 4: offset
    "\x1D\x00\x00\x00" // 8: length
    "this should be overwritten!!\x00"
    // ^ 12: data stored at heap pos 0
    "\x00\x00\x00\x00" // 41: pad to %8
    "\x00\x00\x00"

    "\x00\x00\x00\x00" // 48: memdix
    "\x1D\x00\x00\x00" // 52: offset
    "\x23\x00\x00\x00" // 56: length
    "hello again from sandbox_native.c!\x00"
    // ^ 60: data stored at heap pos 48
    "\x00" // 95: pad to %8

    "\x00\x00\x00\x00" // 96: memdix
    "\x00\x00\x00\x00" // 100: offset (overwrites first segment)
    "\x1D\x00\x00\x00" // 104: length
    "hello from sandbox_native.c!\x00"
    // ^ 108: data stored at heap pos 0
    "\x00\x00\x00\x00" // 149: pad to %8
    "\x00\x00";        // N.b. C will append a null byte

const uint32_t wasm_data_segments_len = UINT_MAX;

void guest_main(void *ctx) {}

int main(void)
{
    return 0;
}
