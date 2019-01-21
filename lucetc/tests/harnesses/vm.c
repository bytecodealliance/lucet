#include "vm.h"
#include <assert.h>
#include <stddef.h>
#include <string.h>

static struct VM vm;

struct VM *make_vm(void)
{
    memset(vm.heap, 0, HEAP_SIZE);
    memset(vm.heap, 0, GLOBALS_SIZE * sizeof(int64_t));
    vm.global_ptr = vm.globals;
    return &vm;
}

struct vmctx *get_vmctx(struct VM *the_vm)
{
    return (struct vmctx *) &the_vm->heap;
}

struct VM *get_vm(struct vmctx *the_vmctx)
{
    assert(the_vmctx == get_vmctx(&vm));
    return &vm;
}

uint32_t lucet_vmctx_grow_memory(struct vmctx *ctx, uint32_t pages) __attribute__((weak));
uint32_t lucet_vmctx_grow_memory(struct vmctx *ctx, uint32_t pages)
{
    assert(get_vm(ctx) == &vm);
    (void) pages;
    return 0;
}

uint32_t lucet_vmctx_current_memory(struct vmctx *ctx) __attribute__((weak));
uint32_t lucet_vmctx_current_memory(struct vmctx *ctx)
{
    assert(get_vm(ctx) == &vm);
    return 1;
}
