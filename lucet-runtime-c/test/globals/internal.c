#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../helpers.h"
#include "../src/lucet_vmctx_private.h"

DEFINE_DEFAULT_HEAP_SPEC;
DEFINE_DEFAULT_DATA_SEGMENTS;
DEFINE_DEFAULT_SPARSE_PAGE_DATA;

// Note: we can't use the struct initializers from lucet_globals_private.h here
// because the serialization scheme requires the spec header and the
// descriptors to be laid out sequentially in memory.
int64_t lucet_globals_spec[] = {
    2, //.num_globals = 2,
    // descriptor[0]:
    0,  // flags: Internal def, no name
    -1, // initial value
    0,  // No name

    // descriptor[0]:
    0,   // flags: Internal def, no name
    420, // initial value
    0,   // No name
};

int64_t guest_func_get_global0(struct lucet_vmctx *ctx)
{
    int64_t *globals = lucet_vmctx_get_globals(ctx);

    return globals[0];
}

int64_t guest_func_get_global1(struct lucet_vmctx *ctx)
{
    int64_t *globals = lucet_vmctx_get_globals(ctx);

    return globals[1];
}

void guest_func_set_global0(struct lucet_vmctx *ctx, int64_t val)
{
    int64_t *globals = lucet_vmctx_get_globals(ctx);

    globals[0] = val;
}

void guest_func_set_global1(struct lucet_vmctx *ctx, int64_t val)
{
    int64_t *globals = lucet_vmctx_get_globals(ctx);

    globals[1] = val;
}
