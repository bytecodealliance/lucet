#include "common.h"

guest_int builtin_strcmp(const struct lucet_vmctx *ctx, guest_ptr_t a_off_, guest_ptr_t b_off_)
{
    char *const  heap            = LUCET_HEAP(ctx);
    const size_t heap_size_bytes = LUCET_CURRENT_HEAP_SIZE(ctx);
    size_t       a_off           = (size_t) a_off_;
    size_t       b_off           = (size_t) b_off_;

    TRAPIF(a_off >= heap_size_bytes || b_off >= heap_size_bytes);
    while (heap[a_off] == heap[b_off]) {
        if (heap[a_off] == 0) {
            return 0;
        }
        a_off++;
        b_off++;
        TRAPIF(a_off >= heap_size_bytes || b_off >= heap_size_bytes);
    }
    return (guest_int)(heap[a_off] - heap[b_off]);
}
