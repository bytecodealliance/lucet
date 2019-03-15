#include "common.h"

uint64_t builtin_memcpy(const struct lucet_vmctx *ctx, uint64_t dst_off, uint64_t src_off,
                        uint64_t len)
{
    char *const  heap            = LUCET_HEAP(ctx);
    const size_t heap_size_bytes = LUCET_CURRENT_HEAP_SIZE(ctx);

    TRAPIF(src_off >= heap_size_bytes || dst_off >= heap_size_bytes);
    TRAPIF(heap_size_bytes - src_off < len || heap_size_bytes - dst_off < len);

    const char *const src = heap + src_off;
    char *const       dst = heap + dst_off;

    memcpy(dst, src, (size_t) len);

    return dst_off;
}
