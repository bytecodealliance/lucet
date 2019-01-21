#include <assert.h>
#include <string.h>

#include "lucet_data_segment_private.h"

/**
 * Check that a data segment descriptor fits in the given spec. Returns true
 * when valid, false otherwise.
 */
bool lucet_data_segment_validate(struct lucet_data_segment_descriptor const *d,
                                 struct lucet_alloc_heap_spec const *        spec)
{
    uint32_t p_segs = 0; // pos in data segment to copy from
    while (p_segs < d->len) {
        struct lucet_data_segment *seg =
            (struct lucet_data_segment *) ((uintptr_t) d->segments + (uintptr_t) p_segs);
        const uint32_t lm_end = seg->offset + seg->length;

        if (lm_end > spec->initial_size || lm_end < seg->offset) {
            return false;
        }

        p_segs += sizeof(struct lucet_data_segment) + seg->length;
        p_segs += (8 - p_segs % 8) % 8; // pad to 8
    }
    return true;
}
/**
 * Initialize a heap, given a data segment descriptor.
 * Precondition: heap spec associated with heap has passed
 * lucet_data_segment_validate.
 */
void lucet_data_segment_initialize_heap(struct lucet_data_segment_descriptor const *d,
                                        struct lucet_alloc *                        a)
{
    // Iterate over data segments and copy them to linear memory
    uint32_t p_segs = 0; // pos in data segment to copy from
    while (p_segs < d->len) {
        struct lucet_data_segment *seg =
            (struct lucet_data_segment *) ((uintptr_t) d->segments + (uintptr_t) p_segs);

        memcpy(a->heap + seg->offset, seg->data, seg->length);

        p_segs += sizeof(struct lucet_data_segment) + seg->length;
        p_segs += (8 - p_segs % 8) % 8; // pad to 8
    }
}
