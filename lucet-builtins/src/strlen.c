#include "common.h"

guest_size_t builtin_strlen(const struct lucet_vmctx *ctx, guest_ptr_t str_off)
{
    char *const  heap            = LUCET_HEAP(ctx);
    const size_t heap_size_bytes = LUCET_CURRENT_HEAP_SIZE(ctx);
    size_t       left;
    uint64_t     t;
    guest_ptr_t  str_save = str_off;

    TRAPIF((size_t) str_off > heap_size_bytes);
    left = heap_size_bytes - (size_t) str_off;

    while (left >= 8U) {
        memcpy(&t, &heap[str_off], 8);
        if ((t & (uint64_t) 0xffULL) == (uint64_t) 0U) {
            return str_off - str_save;
        }
        if ((t & (uint64_t) 0xff00ULL) == (uint64_t) 0U) {
            return str_off - str_save + 1U;
        }
        if ((t & (uint64_t) 0xff0000ULL) == (uint64_t) 0U) {
            return str_off - str_save + 2U;
        }
        if ((t & (uint64_t) 0xff000000ULL) == (uint64_t) 0U) {
            return str_off - str_save + 3U;
        }
        if ((t & (uint64_t) 0xff00000000ULL) == (uint64_t) 0U) {
            return str_off - str_save + 4U;
        }
        if ((t & (uint64_t) 0xff0000000000ULL) == (uint64_t) 0U) {
            return str_off - str_save + 5U;
        }
        if ((t & (uint64_t) 0xff000000000000ULL) == (uint64_t) 0U) {
            return str_off - str_save + 6U;
        }
        if ((t & (uint64_t) 0xff00000000000000ULL) == (uint64_t) 0U) {
            return str_off - str_save + 7U;
        }
        str_off += 8U;
        left -= 8U;
    }
    while (left > 0U) {
        if (heap[str_off] == 0) {
            return str_off - str_save;
        }
        str_off++;
        left--;
    }
    TRAP;
    /* UNREACHABLE */
    return 0;
}
