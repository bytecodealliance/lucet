#include "common.h"

guest_ptr_t builtin_strchr(const struct lucet_vmctx *ctx, guest_ptr_t str_off, int c)
{
    char *const    heap            = LUCET_HEAP(ctx);
    const size_t   heap_size_bytes = LUCET_CURRENT_HEAP_SIZE(ctx);
    const uint64_t ones            = 0x0101010101010101;
    size_t         left;
    uint64_t       cs;
    uint64_t       t, u;

    TRAPIF((size_t) str_off > heap_size_bytes);
    left = heap_size_bytes - (size_t) str_off;
    cs   = (uint64_t) c;
    cs |= (cs << 8);
    cs |= (cs << 16);
    cs |= (cs << 32);

    while (left >= 8U) {
        memcpy(&t, &heap[str_off], 8);
        u = t ^ cs;
        if (((((t - ones) & ~t) | ((u - ones) & ~u)) & (ones << 7)) != 0U) {
            break;
        }
        str_off += 8U;
        left -= 8U;
    }
    while (left > 0U) {
        if (heap[str_off] == (char) c) {
            return str_off;
        }
        if (heap[str_off] == (char) 0) {
            return (guest_ptr_t) 0U;
        }
        str_off++;
        left--;
    }
    TRAP;
    /* UNREACHABLE */
    return (guest_ptr_t) 0U;
}
