
#include "lucet_vmctx.h"
#include "lucet_alloc_private.h"
#include "lucet_instance_private.h"
#include "lucet_module_private.h"
#include "lucet_vmctx_private.h"

// Get a pointer to the instance heap.
char *lucet_vmctx_get_heap(struct lucet_vmctx const *ctx)
{
    return (char *) ctx;
}

// Check if a memory region is inside the instance heap.
bool lucet_vmctx_check_heap(struct lucet_vmctx const *ctx, void *ptr, size_t len)
{
    struct lucet_instance *inst = lucet_vmctx_instance(ctx);
    return lucet_instance_check_heap(inst, ptr, len);
}

void lucet_vmctx_terminate(struct lucet_vmctx const *ctx, void *info)
{
    struct lucet_instance *inst = lucet_vmctx_instance(ctx);
    lucet_instance_terminate(inst, info);
}

// Get the delegate object for a given instance
void *lucet_vmctx_get_delegate(struct lucet_vmctx const *ctx)
{
    struct lucet_instance *inst = lucet_vmctx_instance(ctx);
    return lucet_instance_get_delegate(inst);
}

// returns the current number of wasm pages
uint32_t lucet_vmctx_current_memory(struct lucet_vmctx const *ctx)
{
    struct lucet_instance const *inst = lucet_vmctx_instance(ctx);
    return lucet_instance_current_memory(inst);
}

// takes the number of wasm pages to grow by. returns the number of pages before
// the call on success, or -1 on failure.
int32_t lucet_vmctx_grow_memory(struct lucet_vmctx const *ctx, uint32_t additional_pages)
{
    struct lucet_instance *inst = lucet_vmctx_instance(ctx);
    return lucet_instance_grow_memory(inst, additional_pages);
}

// returns the address of a function given its ID
void *lucet_vmctx_get_func_from_id(struct lucet_vmctx const *ctx, uint32_t table_id,
                                   uint32_t func_id)
{
    struct lucet_instance const *inst   = lucet_vmctx_instance(ctx);
    struct lucet_module const *  module = inst->module;
    return lucet_module_get_func_from_id(module, table_id, func_id);
}

// Mostly for tests - this conversion is builtin to lucetc
int64_t *lucet_vmctx_get_globals(struct lucet_vmctx const *ctx)
{
    return *(int64_t **) (&((char *) ctx)[-sizeof(void *)]);
}
