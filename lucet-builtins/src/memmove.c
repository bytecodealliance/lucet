#include "common.h"

guest_ptr_t builtin_memmove(const struct lucet_vmctx *ctx, guest_ptr_t dst_off, guest_ptr_t src_off,
                            guest_size_t len)
{
    char *const  heap            = LUCET_HEAP(ctx);
    const size_t heap_size_bytes = LUCET_CURRENT_HEAP_SIZE(ctx);

    TRAPIF(src_off >= heap_size_bytes || dst_off >= heap_size_bytes);
    TRAPIF(heap_size_bytes - src_off < len || heap_size_bytes - dst_off < len);

    const char *const src = heap + src_off;
    char *const       dst = heap + dst_off;

    memmove(dst, src, (size_t) len);

    return dst_off;
}
