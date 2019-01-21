#include <stddef.h>
#include <stdio.h>

#include <lucet.h>

void
sg_log(struct lucet_vmctx *ctx, guest_ptr_t msg_ptr)
{
    char *heap = lucet_vmctx_get_heap(ctx);

    const char *msg = (const char *) &heap[msg_ptr];
    printf("* DEBUG: [%s]\n", msg);
}

void
sg_black_box(void *x)
{
    (void) x;
}
