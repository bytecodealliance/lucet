#include <assert.h>
#include <inttypes.h>

#include "greatest.h"

#include "../src/lucet_alloc_private.h"
#include "../src/lucet_instance_private.h"
#include "../src/lucet_module_private.h"

struct lucet_globals_spec no_globals = {
    .num_globals = 0,
};

#define LIMITS_HEAP_MEM_SIZE (16 * 64 * 1024)
#define LIMITS_HEAP_ADDRSPACE_SIZE (8 * 1024 * 1024)
#define LIMITS_STACK_SIZE (64 * 1024)
#define LIMITS_GLOBALS_SIZE (4 * 1024)
struct lucet_alloc_limits limits = {
    .heap_memory_size        = LIMITS_HEAP_MEM_SIZE,
    .heap_address_space_size = LIMITS_HEAP_ADDRSPACE_SIZE,
    .stack_size              = LIMITS_STACK_SIZE,
    .globals_size            = LIMITS_GLOBALS_SIZE,
};

#define SPEC_HEAP_RESERVED_SIZE (LIMITS_HEAP_ADDRSPACE_SIZE / 2)
#define SPEC_HEAP_GUARD_SIZE (LIMITS_HEAP_ADDRSPACE_SIZE / 2)

// one page meaning one wasm page, which are 64k:
#define ONEPAGE_INITIAL_SIZE (64 * 1024)
#define ONEPAGE_MAX_SIZE (64 * 1024)
struct lucet_alloc_heap_spec h_one_page_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = SPEC_HEAP_GUARD_SIZE,
    .initial_size   = ONEPAGE_INITIAL_SIZE,
    .max_size       = ONEPAGE_MAX_SIZE,
    .max_size_valid = 1,
};

struct lucet_alloc_runtime_spec one_page_heap = {
    .heap    = &h_one_page_heap,
    .globals = &no_globals,
};

#define THREEPAGE_INITIAL_SIZE (64 * 1024)
#define THREEPAGE_MAX_SIZE (3 * 64 * 1024)
struct lucet_alloc_heap_spec h_three_page_max_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = 0,
    .initial_size   = THREEPAGE_INITIAL_SIZE,
    .max_size       = THREEPAGE_MAX_SIZE,
    .max_size_valid = 1,
};

struct lucet_alloc_runtime_spec three_page_max_heap = {
    .heap    = &h_three_page_max_heap,
    .globals = &no_globals,
};

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

TEST alloc_create(void)
{
    // This test shows we can use alloc to create an instance, and
    // manipulate it right away (without doing an allocate_runtime)
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);
    struct lucet_instance *    inst   = lucet_alloc_get_instance(alloc);

    ASSERT(inst);
    // The lucet_alloc call never actually touches the inst - thats left entrely
    // up to lucet_instance.c. So, initially this pointer is null because the
    // instance is empty.
    ASSERT_EQ(inst->alloc, NULL);
    // It is readable and writable:
    inst->alloc = alloc;
    ASSERT_EQ(inst->alloc, alloc);

    lucet_alloc_free_region(region);
    PASS();
}

TEST heap_create(void)
{
    // This test shows an alloc, when used to create a heap, will
    // have the size given in the spec, and be read-writeable in that
    // region. Same with the stack.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &one_page_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, ONEPAGE_INITIAL_SIZE);
    char *heap = lucet_alloc_get_heap(alloc);

    ASSERT_EQ(heap[0], 0);
    heap[0] = -1;
    ASSERT_EQ(heap[0], -1);
    ASSERT_EQ(heap[heap_len - 1], 0);
    heap[heap_len - 1] = -1;
    ASSERT_EQ(heap[heap_len - 1], -1);

    char *sp = lucet_alloc_get_stack_top(alloc);
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], 0);
    sp[-LIMITS_STACK_SIZE] = -1;
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], -1);
    ASSERT_EQ(sp[-1], 0);
    sp[-1] = -1;
    ASSERT_EQ(sp[-1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

TEST expand_heap_once(void)
{
    // This test shows an alloc heap works properly after a single expand.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &three_page_max_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, THREEPAGE_INITIAL_SIZE);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);

    ASSERT_EQ(heap_len, new_heap_area);

    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(heap_len + (64 * 1024), new_heap_len);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[new_heap_len - 1], 0);
    heap[new_heap_len - 1] = -1;
    ASSERT_EQ(heap[new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

TEST expand_heap_twice(void)
{
    // This test shows an alloc heap works properly after two expands.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &three_page_max_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, THREEPAGE_INITIAL_SIZE);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);
    ASSERT_EQ(heap_len, new_heap_area);
    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len + (64 * 1024), new_heap_len);

    uint32_t second_new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);
    ASSERT_EQ(new_heap_len, second_new_heap_area);
    uint32_t second_new_heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(second_new_heap_len, THREEPAGE_MAX_SIZE);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[second_new_heap_len - 1], 0);
    heap[second_new_heap_len - 1] = -1;
    ASSERT_EQ(heap[second_new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

TEST expand_past_spec_max(void)
{
    // This test shows that, if you try to expand past the max given by the spec
    // for a heap, the
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &three_page_max_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, THREEPAGE_INITIAL_SIZE);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, THREEPAGE_MAX_SIZE);

    ASSERT_EQ(-1, new_heap_area);

    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(heap_len, new_heap_len);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[new_heap_len - 1], 0);
    heap[new_heap_len - 1] = -1;
    ASSERT_EQ(heap[new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

#define EXPANDPASTLIMIT_INITIAL_SIZE (LIMITS_HEAP_MEM_SIZE - (64 * 1024))
#define EXPANDPASTLIMIT_MAX_SIZE (LIMITS_HEAP_MEM_SIZE + (64 * 1024))
struct lucet_alloc_heap_spec h_expand_past_limit_spec = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = SPEC_HEAP_GUARD_SIZE,
    .initial_size   = EXPANDPASTLIMIT_INITIAL_SIZE,
    .max_size       = EXPANDPASTLIMIT_MAX_SIZE,
    .max_size_valid = 1,
};
struct lucet_alloc_runtime_spec expand_past_limit_spec = {
    .heap    = &h_expand_past_limit_spec,
    .globals = &no_globals,
};

TEST expand_past_heap_limit(void)
{
    // This test shows that a heap refuses to grow past the alloc limits,
    // even if the spec says it can grow bigger.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &expand_past_limit_spec);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, EXPANDPASTLIMIT_INITIAL_SIZE);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);

    ASSERT_EQ(heap_len, new_heap_area);

    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(LIMITS_HEAP_MEM_SIZE, new_heap_len);

    uint32_t past_limit_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);

    ASSERT_EQ(-1, past_limit_heap_area);

    uint32_t still_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(LIMITS_HEAP_MEM_SIZE, still_heap_len);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[new_heap_len - 1], 0);
    heap[new_heap_len - 1] = -1;
    ASSERT_EQ(heap[new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}
struct lucet_alloc_heap_spec h_initial_oversize_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = SPEC_HEAP_GUARD_SIZE,
    .initial_size   = SPEC_HEAP_RESERVED_SIZE + (64 * 1024),
    .max_size       = 0,
    .max_size_valid = 0,
};
struct lucet_alloc_runtime_spec initial_oversize_heap = {
    .heap    = &h_initial_oversize_heap,
    .globals = &no_globals,
};

TEST reject_initial_oversize_heap(void)
{
    // This test shows that alloc will fail on a spec that has an
    // initial size over the limits.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &initial_oversize_heap);
    ASSERT_ENUM_EQ(lucet_alloc_spec_over_limits, stat, lucet_alloc_stat_name);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, 0);

    lucet_alloc_free_region(region);
    PASS();
}

struct lucet_alloc_heap_spec h_small_guard_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = SPEC_HEAP_GUARD_SIZE - 1,
    .initial_size   = LIMITS_HEAP_MEM_SIZE,
    .max_size       = 0,
    .max_size_valid = 0,
};
struct lucet_alloc_runtime_spec small_guard_heap = {
    .heap    = &h_small_guard_heap,
    .globals = &no_globals,
};

TEST accept_small_guard_heap(void)
{
    // This test shows that a heap spec with a guard size smaller than the
    // limits is allowed.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &small_guard_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);

    lucet_alloc_free_region(region);
    PASS();
}

struct lucet_alloc_heap_spec h_large_guard_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = SPEC_HEAP_GUARD_SIZE + 1,
    .initial_size   = ONEPAGE_INITIAL_SIZE,
    .max_size       = 0,
    .max_size_valid = 0,
};
struct lucet_alloc_runtime_spec large_guard_heap = {
    .heap    = &h_large_guard_heap,
    .globals = &no_globals,
};

TEST reject_large_guard_heap(void)
{
    // This test shows that a heap spec with a guard size larger than the limits
    // is not allowed.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &large_guard_heap);
    ASSERT_ENUM_EQ(lucet_alloc_spec_over_limits, stat, lucet_alloc_stat_name);

    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, 0);

    lucet_alloc_free_region(region);
    PASS();
}

TEST runtime_reallocate(void)
{
    // This test shows that if you free_runtime then allocate_runtime, the heap,
    // stack, and globals are clear and work as before.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &one_page_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    char *heap = lucet_alloc_get_heap(alloc);

    ASSERT_EQ(heap[0], 0);
    heap[0] = -1;
    ASSERT_EQ(heap[0], -1);
    ASSERT_EQ(heap[ONEPAGE_INITIAL_SIZE - 1], 0);
    heap[ONEPAGE_INITIAL_SIZE - 1] = -1;
    ASSERT_EQ(heap[ONEPAGE_INITIAL_SIZE - 1], -1);

    char *sp = lucet_alloc_get_stack_top(alloc);
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], 0);
    sp[-LIMITS_STACK_SIZE] = -1;
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], -1);
    ASSERT_EQ(sp[-1], 0);
    sp[-1] = -1;
    ASSERT_EQ(sp[-1], -1);

    char *globals = lucet_alloc_get_globals(alloc);
    ASSERT_EQ(globals[0], 0);
    globals[0] = -1;
    ASSERT_EQ(globals[0], -1);
    ASSERT_EQ(globals[LIMITS_GLOBALS_SIZE - 1], 0);
    globals[LIMITS_GLOBALS_SIZE - 1] = -1;
    ASSERT_EQ(globals[LIMITS_GLOBALS_SIZE - 1], -1);

    stack_t ss;
    lucet_alloc_get_sigstack(alloc, &ss);
    char *sigstack = (char *) ss.ss_sp;
    ASSERT_EQ(sigstack[0], 0);
    sigstack[0] = -1;
    ASSERT_EQ(sigstack[0], -1);
    ASSERT_EQ(sigstack[ss.ss_size - 1], 0);
    sigstack[ss.ss_size - 1] = -1;
    ASSERT_EQ(sigstack[ss.ss_size - 1], -1);

    lucet_alloc_free_runtime(alloc);

    enum lucet_alloc_stat const stat2 = lucet_alloc_allocate_runtime(alloc, &one_page_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat2, lucet_alloc_stat_name);

    // Mock the instance and module
    inst->module = &module_with_no_datainit;
    inst->alloc  = alloc;

    ASSERT_EQ(heap[ONEPAGE_INITIAL_SIZE - 1], 0);
    heap[ONEPAGE_INITIAL_SIZE - 1] = -1;
    ASSERT_EQ(heap[ONEPAGE_INITIAL_SIZE - 1], -1);

    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], 0);
    sp[-LIMITS_STACK_SIZE] = -1;
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], -1);
    ASSERT_EQ(sp[-1], 0);
    sp[-1] = -1;
    ASSERT_EQ(sp[-1], -1);

    ASSERT_EQ(globals[0], 0);
    globals[0] = -1;
    ASSERT_EQ(globals[0], -1);
    ASSERT_EQ(globals[LIMITS_GLOBALS_SIZE - 1], 0);
    globals[LIMITS_GLOBALS_SIZE - 1] = -1;
    ASSERT_EQ(globals[LIMITS_GLOBALS_SIZE - 1], -1);

    ASSERT_EQ(sigstack[0], 0);
    sigstack[0] = -1;
    ASSERT_EQ(sigstack[0], -1);
    ASSERT_EQ(sigstack[ss.ss_size - 1], 0);
    sigstack[ss.ss_size - 1] = -1;
    ASSERT_EQ(sigstack[ss.ss_size - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

TEST runtime_reset(void)
{
    // This test shows that the reset method clears the heap and restores it to
    // the spec initial size.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &three_page_max_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    char *heap = lucet_alloc_get_heap(alloc);

    ASSERT_EQ(heap[0], 0);
    heap[0] = -1;
    ASSERT_EQ(heap[0], -1);
    ASSERT_EQ(heap[THREEPAGE_INITIAL_SIZE - 1], 0);
    heap[THREEPAGE_INITIAL_SIZE - 1] = -1;
    ASSERT_EQ(heap[THREEPAGE_INITIAL_SIZE - 1], -1);

    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, THREEPAGE_INITIAL_SIZE);

    uint32_t new_heap_area =
        lucet_alloc_expand_heap(alloc, THREEPAGE_MAX_SIZE - THREEPAGE_INITIAL_SIZE);
    ASSERT_EQ(heap_len, new_heap_area);
    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(THREEPAGE_MAX_SIZE, new_heap_len);

    ASSERT_EQ(heap[THREEPAGE_MAX_SIZE - 1], 0);
    heap[THREEPAGE_MAX_SIZE - 1] = -1;
    ASSERT_EQ(heap[THREEPAGE_MAX_SIZE - 1], -1);

    lucet_alloc_reset_runtime(alloc, &module_with_no_datainit);

    uint32_t reset_heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(THREEPAGE_INITIAL_SIZE, reset_heap_len);

    ASSERT_EQ(heap[0], 0);
    heap[0] = -1;
    ASSERT_EQ(heap[0], -1);
    ASSERT_EQ(heap[THREEPAGE_INITIAL_SIZE - 1], 0);
    heap[THREEPAGE_INITIAL_SIZE - 1] = -1;
    ASSERT_EQ(heap[THREEPAGE_INITIAL_SIZE - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);

    PASS();
}

struct lucet_alloc_heap_spec h_guardless_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = 0,
    .initial_size   = ONEPAGE_INITIAL_SIZE,
    .max_size       = 0,
    .max_size_valid = 0,
};

struct lucet_alloc_runtime_spec guardless_heap = {
    .heap    = &h_guardless_heap,
    .globals = &no_globals,
};

TEST guardless_heap_create(void)
{
    // This test shows an alloc, when used to create a heap, will
    // have the size given in the spec, and be read-writeable in that
    // region. Same with the stack.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &guardless_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, ONEPAGE_INITIAL_SIZE);
    char *heap = lucet_alloc_get_heap(alloc);

    ASSERT_EQ(heap[0], 0);
    heap[0] = -1;
    ASSERT_EQ(heap[0], -1);
    ASSERT_EQ(heap[heap_len - 1], 0);
    heap[heap_len - 1] = -1;
    ASSERT_EQ(heap[heap_len - 1], -1);

    char *sp = lucet_alloc_get_stack_top(alloc);
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], 0);
    sp[-LIMITS_STACK_SIZE] = -1;
    ASSERT_EQ(sp[-LIMITS_STACK_SIZE], -1);
    ASSERT_EQ(sp[-1], 0);
    sp[-1] = -1;
    ASSERT_EQ(sp[-1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

TEST guardless_expand_heap_once(void)
{
    // This test shows an alloc heap works properly after a single expand.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &guardless_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, ONEPAGE_INITIAL_SIZE);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);

    ASSERT_EQ(heap_len, new_heap_area);

    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(heap_len + (64 * 1024), new_heap_len);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[new_heap_len - 1], 0);
    heap[new_heap_len - 1] = -1;
    ASSERT_EQ(heap[new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

struct lucet_alloc_heap_spec h_initial_empty_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = SPEC_HEAP_GUARD_SIZE,
    .initial_size   = 0,
    .max_size       = 0,
    .max_size_valid = 0,
};

struct lucet_alloc_runtime_spec initial_empty_heap = {
    .heap    = &h_initial_empty_heap,
    .globals = &no_globals,
};

TEST initial_empty_expand_heap_once(void)
{
    // This test shows an alloc heap works properly after a single expand.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &initial_empty_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, 0);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);

    ASSERT_EQ(heap_len, new_heap_area);

    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(heap_len + (64 * 1024), new_heap_len);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[new_heap_len - 1], 0);
    heap[new_heap_len - 1] = -1;
    ASSERT_EQ(heap[new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

struct lucet_alloc_heap_spec h_initial_empty_guardless_heap = {
    .reserved_size  = SPEC_HEAP_RESERVED_SIZE,
    .guard_size     = 0,
    .initial_size   = 0,
    .max_size       = 0,
    .max_size_valid = 0,
};

struct lucet_alloc_runtime_spec initial_empty_guardless_heap = {
    .heap    = &h_initial_empty_guardless_heap,
    .globals = &no_globals,
};

TEST initial_empty_guardless_expand_heap_once(void)
{
    // This test shows an alloc heap works properly after a single expand.
    struct lucet_alloc_region *region = lucet_alloc_create_region(1, &limits);
    struct lucet_alloc *       alloc  = lucet_alloc_region_get_alloc(region, 0);

    enum lucet_alloc_stat const stat =
        lucet_alloc_allocate_runtime(alloc, &initial_empty_guardless_heap);
    ASSERT_ENUM_EQ(lucet_alloc_ok, stat, lucet_alloc_stat_name);
    uint32_t heap_len = lucet_alloc_get_heap_len(alloc);
    ASSERT_EQ(heap_len, 0);

    // Mock the instance and module
    struct lucet_instance *inst = lucet_alloc_get_instance(alloc);
    inst->magic                 = LUCET_INSTANCE_MAGIC;
    inst->module                = &module_with_no_datainit;
    inst->alloc                 = alloc;

    uint32_t new_heap_area = lucet_alloc_expand_heap(alloc, 64 * 1024);

    ASSERT_EQ(heap_len, new_heap_area);

    uint32_t new_heap_len = lucet_alloc_get_heap_len(alloc);

    ASSERT_EQ(heap_len + (64 * 1024), new_heap_len);

    char *heap = lucet_alloc_get_heap(alloc);
    ASSERT_EQ(heap[new_heap_len - 1], 0);
    heap[new_heap_len - 1] = -1;
    ASSERT_EQ(heap[new_heap_len - 1], -1);

    lucet_alloc_free_runtime(alloc);
    lucet_alloc_free_region(region);
    PASS();
}

SUITE(alloc_suite)
{
    RUN_TEST(alloc_create);
    RUN_TEST(heap_create);
    RUN_TEST(expand_heap_once);
    RUN_TEST(expand_heap_twice);
    RUN_TEST(expand_past_spec_max);
    RUN_TEST(expand_past_heap_limit);
    RUN_TEST(reject_initial_oversize_heap);
    RUN_TEST(accept_small_guard_heap);
    RUN_TEST(reject_large_guard_heap);
    RUN_TEST(runtime_reallocate);
    RUN_TEST(runtime_reset);
    RUN_TEST(guardless_heap_create);
    RUN_TEST(guardless_expand_heap_once);
    RUN_TEST(initial_empty_expand_heap_once);
    RUN_TEST(initial_empty_guardless_expand_heap_once);
}
