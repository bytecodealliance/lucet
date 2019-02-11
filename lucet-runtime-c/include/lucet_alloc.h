#ifndef LUCET_ALLOC_H
#define LUCET_ALLOC_H

#include <stdint.h>

/**
 * All of these limits must be divisible by host page size (4k)
 */
struct lucet_alloc_limits {
    /**
     * Max size of the heap, which can be backed by real memory, in bytes.
     */
    uint64_t heap_memory_size;
    /**
     * Size of total virtual memory
     */
    uint64_t heap_address_space_size;
    /**
     * Size of the stack used by the guest
     */
    uint32_t stack_size;
    /**
     * Size of the globals region, in bytes. Each global uses 8 bytes.
     */
    uint32_t globals_size;
};

extern const struct lucet_alloc_limits lucet_alloc_limits_default;

#endif // LUCET_ALLOC_H
