#include "lucet_alloc_private.h"

const struct lucet_alloc_limits lucet_alloc_limits_default = (struct lucet_alloc_limits){
    .heap_memory_size        = 16 * 64 * 1024, // 16 wasm pages
    .heap_address_space_size = 0x200000000,    // 8gb total (4gb reserved + 4gb guard)
    .stack_size              = 128 * 1024,
    .globals_size            = 4096,
};
