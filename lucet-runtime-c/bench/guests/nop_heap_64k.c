#include "../../src/lucet_alloc_private.h"
#include "lucet_vmctx.h"
#include "mem_utils.h"
#include <inttypes.h>
#include <stdio.h>

struct lucet_alloc_heap_spec lucet_heap_spec = {
    .guard_size     = 4 * 1024 * 1024,
    .reserved_size  = 4 * 1024 * 1024,
    .initial_size   = 2 * 64 * 1024,
    .max_size       = 2 * 64 * 1024,
    .max_size_valid = 1,
};
struct lucet_globals_spec lucet_globals_spec = {
    .num_globals = 0,
};

__attribute__((visibility("default"))) const char wasm_data_segments[] =
    "\x00\x00\x00\x00" // 0: memdix
    "\x00\x00\x00\x00" // 4: offset=0
    "\x00\x00\x01\x00" // 8: length=65536
    DUMMYBYTES_64K     // 12: 64k bytes of data
    ;

__attribute__((visibility("default"))) const uint32_t wasm_data_segments_len =
    sizeof(wasm_data_segments) - 1; // ignore null byte

int main()
{
    return 0;
}
