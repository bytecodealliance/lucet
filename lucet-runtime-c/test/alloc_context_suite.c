
#include "../src/lucet_alloc_private.h"
#include "../src/lucet_context_private.h"
#include "../src/lucet_instance_private.h"
#include "../src/lucet_module_private.h"
#include "../src/lucet_vmctx_private.h"

#include "greatest.h"

static struct lucet_context parent_regs;
static struct lucet_context child_regs;

static struct lucet_sparse_page_data empty_sparse = {
    .num_pages = 0,
};

static const struct lucet_module module_with_no_datainit = {
    .dl_handle = NULL,
    .fbase     = NULL,
    .data_segment =
        {
            .segments = NULL,
            .len      = 0,
        },
    .runtime_spec =
        {
            .heap    = NULL,
            .globals = NULL,
        },
    .trap_manifest =
        {
            .len     = 0,
            .records = NULL,
        },
    .sparse_page_data = &empty_sparse,
};

void heap_touching_child(struct lucet_vmctx *vmctx)
{
    char *heap = (char *) vmctx;
    heap[0]    = 123;
    heap[4095] = 45;
}

TEST alloc_child(void)
{
    // This test shows that an lucet_alloc'd memory will create a heap and stack,
    // and the child code (presumably running on that stack) can access the
    // heap.
    struct lucet_alloc_limits limits = {
        .heap_memory_size        = 4096,
        .heap_address_space_size = 2 * 4096,
        .stack_size              = 4096,
        .globals_size            = 4096,
    };

    struct lucet_alloc_region *  region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *         alloc  = lucet_alloc_region_get_alloc(region, 0);
    struct lucet_alloc_heap_spec h_spec = {
        .reserved_size  = 4096,
        .guard_size     = 4096,
        .initial_size   = 4096,
        .max_size       = 4096,
        .max_size_valid = 1,
    };
    struct lucet_globals_spec g_spec = {
        .num_globals = 0,
    };
    struct lucet_alloc_runtime_spec spec = {
        .heap    = &h_spec,
        .globals = &g_spec,
    };
    lucet_alloc_allocate_runtime(alloc, &spec);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    lucet_context_init(&child_regs, lucet_alloc_get_stack_top(alloc), &parent_regs,
                       heap_touching_child, (void *) lucet_alloc_get_heap(alloc), 0);

    lucet_context_swap(&parent_regs, &child_regs);

    ASSERT_EQ(lucet_alloc_get_heap(alloc)[0], 123);
    ASSERT_EQ(lucet_alloc_get_heap(alloc)[4095], 45);

    lucet_alloc_free_region(region);

    PASS();
}

void stack_pattern_child(struct lucet_vmctx *vmctx)
{
    uint8_t onthestack[1024];
    for (int i = 0; i < 1024; i++) {
        onthestack[i] = i % 256;
    }
    ((uintptr_t *) vmctx)[0] = (uintptr_t) onthestack;
}

TEST stack_pattern(void)
{
    // This test shows that an lucet_alloc'd memory will create a heap and stack,
    // and the child code can write a pattern to that stack, and we can read
    // back that same pattern after it is done running. Should show pretty
    // definitively that the stack setup works properly.
    struct lucet_alloc_limits limits = {
        .heap_memory_size        = 4096,
        .heap_address_space_size = 2 * 4096,
        .stack_size              = 4096,
        .globals_size            = 0,
    };

    struct lucet_alloc_region *  region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *         alloc  = lucet_alloc_region_get_alloc(region, 0);
    struct lucet_alloc_heap_spec h_spec = {
        .reserved_size  = 4096,
        .guard_size     = 4096,
        .initial_size   = 4096,
        .max_size       = 4096,
        .max_size_valid = 1,
    };
    struct lucet_globals_spec g_spec = {
        .num_globals = 0,
    };
    struct lucet_alloc_runtime_spec spec = {
        .heap    = &h_spec,
        .globals = &g_spec,
    };
    lucet_alloc_allocate_runtime(alloc, &spec);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    lucet_context_init(&child_regs, lucet_alloc_get_stack_top(alloc), &parent_regs,
                       stack_pattern_child, (void *) lucet_alloc_get_heap(alloc), 0);

    lucet_context_swap(&parent_regs, &child_regs);

    uintptr_t stack_pattern = ((uintptr_t *) lucet_alloc_get_heap(alloc))[0];
    ASSERT(stack_pattern > (uintptr_t) alloc->stack);
    ASSERT((stack_pattern + 1024) < (uintptr_t) lucet_alloc_get_stack_top(alloc));

    for (int i = 0; i < 1024; i++) {
        ASSERT_EQ(((uint8_t *) stack_pattern)[i], i % 256);
    }
    lucet_alloc_free_region(region);

    PASS();
}

SUITE(alloc_context_suite)
{
    RUN_TEST(alloc_child);
    RUN_TEST(stack_pattern);
}
