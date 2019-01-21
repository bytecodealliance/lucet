#ifndef LUCET_DATA_SEGMENT_PRIVATE_H
#define LUCET_DATA_SEGMENT_PRIVATE_H

#include "lucet_alloc_private.h"
#include <stdbool.h>
#include <stdint.h>

/*
 * The liblucet/lucetc representation of a WebAssembly data segment [0].
 *
 * Instances of this struct are stored in the shared object produced by the
 * cton-lucet toolchain. They are read by liblucet and used to initialize linear
 * memory when a module is instantiated.
 *
 * Notes:
 *   - An offset can be a constant expression, which can be
 *       - A 32-bit unsigned integer
 *       - A constant retrieved via get_global (TODO: currently unsupported)
 *
 * [0] https://webassembly.github.io/spec/syntax/modules.html#data-segments
 */
struct lucet_data_segment {
    uint32_t memory_index;
    uint32_t offset;
    uint32_t length;
    char     data[];
};

struct lucet_data_segment_descriptor {
    void *   segments;
    uint32_t len;
};

/**
 * Check that a data segment descriptor fits in the given spec. Returns true
 * when valid, false otherwise.
 */
bool lucet_data_segment_validate(struct lucet_data_segment_descriptor const *,
                                 struct lucet_alloc_heap_spec const *);

/**
 * Initialize a heap, given a data segment descriptor.
 * Precondition: heap spec associated with heap has passed
 * lucet_data_segment_validate.
 */
void lucet_data_segment_initialize_heap(struct lucet_data_segment_descriptor const *,
                                        struct lucet_alloc *);

#endif // LUCET_DATA_SEGMENT_PRIVATE_H
