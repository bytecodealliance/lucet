
#include <stdint.h>

struct lucet_heap_spec {
    uint64_t reserved_size;
    uint64_t guard_size;
    uint64_t initial_size;
    uint64_t max_size;
    uint64_t max_size_valid; // Just a boolean
};

extern struct lucet_heap_spec lucet_heap_spec;
