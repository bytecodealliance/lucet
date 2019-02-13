#include <assert.h>
#include <err.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#include "lucet_alloc_private.h"
#include "lucet_constants.h"
#include "lucet_data_segment_private.h"
#include "lucet_instance_private.h"
#include "lucet_module_private.h"

// Host is Linux x64, which should always use a 4k page. We have an assertion
// to check that sysconf agrees with this assumption below in
// lucet_alloc_allocate_instance.
#define HOST_PAGE_SIZE 4096
#define INSTANCE_HEAP_OFFSET (1 * HOST_PAGE_SIZE)

_Static_assert((sizeof(struct lucet_instance)) == INSTANCE_HEAP_OFFSET,
               "instance size is heap offset");
const size_t lucet_alloc_instance_heap_offset = INSTANCE_HEAP_OFFSET;

static void lucet_alloc_create(struct lucet_alloc *a, struct lucet_alloc_region *region);
static void lucet_alloc_free(struct lucet_alloc *a);

static size_t total_mmap_size(struct lucet_alloc_limits const *limits)
{
    assert(limits);
    // The OS should always recommend a signal stack size in page increments
    // but we will make sure:
    // By default x64 Linux uses 8k.
    assert((SIGSTKSZ % HOST_PAGE_SIZE) == 0);

    // Memory is laid out as follows:
    // * the instance (up to lucet_alloc_instance_heap_offset)
    // * the heap, followed by guard pages
    // * the stack (grows towards heap guard pages)
    // * one guard page (i guess for good luck?)
    // * globals
    // * one guard page (to catch signal stack overflow)
    // * the signal stack (size given by signal.h SIGSTKSZ macro)
    return lucet_alloc_instance_heap_offset + limits->heap_address_space_size + limits->stack_size +
           HOST_PAGE_SIZE + limits->globals_size + HOST_PAGE_SIZE + SIGSTKSZ;
}

// In this implementation, a region just needs to keep track of limits
struct lucet_alloc_region {
    struct lucet_alloc_limits const *limits;
    int                              size;
    struct lucet_alloc *             allocs;
};

struct lucet_alloc *lucet_alloc_region_get_alloc(struct lucet_alloc_region const *region, int index)
{
    assert(region);
    if (index >= 0 && index < region->size) {
        return &region->allocs[index];
    } else {
        return NULL;
    }
}

struct lucet_alloc_region *lucet_alloc_create_region(int                              num_entries,
                                                     struct lucet_alloc_limits const *limits)
{
    assert(num_entries > 0);
    struct lucet_alloc_region *region = malloc(sizeof(struct lucet_alloc_region));
    if (region == NULL) {
        err(1, "could not allocate region");
    }
    struct lucet_alloc *allocs = calloc(num_entries, sizeof(struct lucet_alloc));
    *region                    = (struct lucet_alloc_region){
        .limits = limits,
        .size   = num_entries,
        .allocs = allocs,
    };
    for (int i = 0; i < num_entries; i++) {
        lucet_alloc_create(&allocs[i], region);
    }
    return region;
}

void lucet_alloc_free_region(struct lucet_alloc_region *region)
{
    assert(region);
    for (int i = 0; i < region->size; i++) {
        lucet_alloc_free(&region->allocs[i]);
    }
    free(region->allocs);
    free(region);
}

static void lucet_alloc_create(struct lucet_alloc *a, struct lucet_alloc_region *region)
{
    struct lucet_alloc_limits const *limits   = region->limits;
    long                             pagesize = sysconf(_SC_PAGESIZE);
    assert(pagesize == HOST_PAGE_SIZE);
    // Make sure the limits are aligned on host pages.
    assert(limits);
    assert((limits->heap_memory_size % HOST_PAGE_SIZE) == 0);
    assert((limits->heap_address_space_size % HOST_PAGE_SIZE) == 0);
    assert((limits->stack_size % HOST_PAGE_SIZE) == 0);
    assert(limits->stack_size > 0);
    assert((limits->globals_size % HOST_PAGE_SIZE) == 0);

    // Get the chunk of virtual memory that the allocation will manage.
    char *mem = mmap(NULL, total_mmap_size(limits), PROT_NONE, MAP_ANONYMOUS | MAP_PRIVATE, 0, 0);
    if (mem == NULL)
        err(1, "%s() mmap failed", __FUNCTION__);

    // Make sure the first part of the memory is read/write so that the
    // lucet_instance can be stored there.
    int res = mprotect(mem, lucet_alloc_instance_heap_offset, PROT_READ | PROT_WRITE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);

    // Lay out sections in the memory:
    char *const heap     = &mem[lucet_alloc_instance_heap_offset];
    char *const stack    = heap + limits->heap_address_space_size;
    char *const globals  = stack + limits->stack_size + HOST_PAGE_SIZE;
    char *const sigstack = globals + HOST_PAGE_SIZE;

    *a = (struct lucet_alloc){
        .start                  = mem,
        .heap                   = heap,
        .heap_accessible_size   = 0,
        .heap_inaccessible_size = limits->heap_address_space_size,
        .stack                  = stack,
        .globals                = globals,
        .sigstack               = sigstack,
        .limits                 = limits,
        .spec                   = NULL,
        .region                 = region,
    };
}

static void lucet_alloc_free(struct lucet_alloc *a)
{
    assert(a);
    int res = munmap(a->start, total_mmap_size(a->limits));
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);
}

// Get a pointer to the lucet_instance stored in the memory managed by the alloc
// struct.
struct lucet_instance *lucet_alloc_get_instance(struct lucet_alloc const *a)
{
    assert(a);
    return (struct lucet_instance *) a->start;
}

char *lucet_alloc_get_stack_top(struct lucet_alloc const *a)
{
    assert(a);
    return &a->stack[a->limits->stack_size];
}

enum lucet_alloc_stat lucet_alloc_allocate_runtime(struct lucet_alloc *                   a,
                                                   struct lucet_alloc_runtime_spec const *spec)
{
    assert(a);
    assert(spec);
    // Assure that the instance is page-aligned.
    assert((uintptr_t) a->heap % HOST_PAGE_SIZE == 0);

    // Assure that the total reserved + guard regions fit in the address space.
    // First check makes sure they fit our 32-bit model, and ensures the second
    // check doesn't overflow.
    if (spec->heap->reserved_size > ((uint64_t) UINT32_MAX + 1) ||
        spec->heap->guard_size > ((uint64_t) UINT32_MAX + 1)) {
        return lucet_alloc_spec_over_limits;
    }
    if (spec->heap->reserved_size + spec->heap->guard_size > a->limits->heap_address_space_size) {
        return lucet_alloc_spec_over_limits;
    }

    if (spec->heap->initial_size > a->limits->heap_memory_size) {
        return lucet_alloc_spec_over_limits;
    }

    if ((spec->globals->num_globals * sizeof(uint64_t)) > a->limits->globals_size) {
        return lucet_alloc_spec_over_limits;
    }
    a->spec = spec;

    // The heap starts one page after the lucet_instance struct.
    int res = mprotect(a->heap, spec->heap->initial_size, PROT_READ | PROT_WRITE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);

    a->heap_accessible_size   = spec->heap->initial_size;
    a->heap_inaccessible_size = a->heap_inaccessible_size - spec->heap->initial_size;

    // Make the stack read/writable
    res = mprotect(a->stack, a->limits->stack_size, PROT_READ | PROT_WRITE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);

    // Make the globals read/writable
    res = mprotect(a->globals, a->limits->globals_size, PROT_READ | PROT_WRITE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);

    // Make the sigstack read/writable
    res = mprotect(a->sigstack, SIGSTKSZ, PROT_READ | PROT_WRITE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);

    return lucet_alloc_ok;
}

void lucet_alloc_reset_runtime(struct lucet_alloc *a, struct lucet_module const *mod)
{
    // Reset the heap to the initial size.
    if (a->heap_accessible_size != a->spec->heap->initial_size) {
        a->heap_accessible_size = a->spec->heap->initial_size;
        a->heap_inaccessible_size =
            a->limits->heap_address_space_size - a->spec->heap->initial_size;

        // Turn off any extra pages.
        int res = mprotect(&a->heap[a->heap_accessible_size], a->heap_inaccessible_size, PROT_NONE);
        if (res != 0)
            err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);
        res = madvise(&a->heap[a->heap_accessible_size], a->heap_inaccessible_size, MADV_DONTNEED);
        if (res != 0)
            err(1, "%s() madvise failed (%d)", __FUNCTION__, res);
    }
    // Zero the heap.
    memset(a->heap, 0, a->heap_accessible_size);

    lucet_data_segment_initialize_heap(&mod->data_segment, a);
}

void lucet_alloc_free_runtime(struct lucet_alloc *a)
{
    assert(a);
    assert((uintptr_t) a->heap % HOST_PAGE_SIZE == 0);
    // Clear and disable access to the heap
    int res = mprotect(a->heap, a->limits->heap_address_space_size, PROT_NONE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);
    res = madvise(a->heap, a->limits->heap_address_space_size, MADV_DONTNEED);
    if (res != 0)
        err(1, "%s() madvise failed (%d)", __FUNCTION__, res);

    // Clear and disable access to the stack
    res = mprotect(a->stack, a->limits->stack_size, PROT_NONE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);
    res = madvise(a->stack, a->limits->stack_size, MADV_DONTNEED);
    if (res != 0)
        err(1, "%s() madvise failed (%d)", __FUNCTION__, res);

    // Clear and disable access to the globals
    res = mprotect(a->globals, a->limits->globals_size, PROT_NONE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);
    res = madvise(a->globals, a->limits->globals_size, MADV_DONTNEED);
    if (res != 0)
        err(1, "%s() madvise failed (%d)", __FUNCTION__, res);

    // Clear and disable access to the sigstack
    res = mprotect(a->sigstack, SIGSTKSZ, PROT_NONE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);
    res = madvise(a->sigstack, SIGSTKSZ, MADV_DONTNEED);
    if (res != 0)
        err(1, "%s() madvise failed (%d)", __FUNCTION__, res);

    a->heap_accessible_size   = 0;
    a->heap_inaccessible_size = a->limits->heap_address_space_size;
}

// Expand the heap by (at least) some number of bytes. Return the offset in the
// heap that the new space starts at. Positive return values, on success will
// always be <UINT32_MAX. Return of -1 indicates failure.
int64_t lucet_alloc_expand_heap(struct lucet_alloc *a, uint32_t expand_bytes)
{
    assert(a->spec);

    // No expansion takes place, but, this is not an error.
    if (expand_bytes == 0) {
        return (int64_t) a->heap_accessible_size;
    }

    if (expand_bytes > UINT32_MAX - (HOST_PAGE_SIZE - 1)) {
        return -1; // Prevent overflow. Never should make heap this big
    }
    // Round up to a page boundary:
    uint32_t expand_pagealigned =
        ((expand_bytes + HOST_PAGE_SIZE - 1) / HOST_PAGE_SIZE) * HOST_PAGE_SIZE;

    // The current accessible size is also on a page boundary:
    assert(a->heap_accessible_size % HOST_PAGE_SIZE == 0);

    // heap_inaccessible_size tracks the size of the allocation that is
    // addressible but not accessible. We cannot perform an expansion larger
    // than this size.
    if (expand_pagealigned > a->heap_inaccessible_size) {
        return -1;
    }
    // The above ensures this expression does not underflow:
    uint32_t guard_remaining = a->heap_inaccessible_size - expand_pagealigned;
    // The compiler specifies how much guard (memory which traps on access) must
    // be beyond the end of the accessible memory. We cannot perform an
    // expansion that would make this region smaller than the compiler expected
    // it to be.
    if (guard_remaining < a->spec->heap->guard_size) {
        return -1;
    }

    // The compiler indicates that the module has specified a maximum memory
    // size. Don't let the heap expand beyond that:
    if (a->spec->heap->max_size_valid &&
        (a->heap_accessible_size + expand_pagealigned) > a->spec->heap->max_size) {
        return -1;
    }

    // The runtime sets a limit on how much of the heap can be backed by
    // real memory. Don't let the heap expand beyond that:
    if ((a->heap_accessible_size + expand_pagealigned) > a->limits->heap_memory_size) {
        return -1;
    }

    uint32_t newly_accessible = a->heap_accessible_size;
    int      res = mprotect(&a->heap[newly_accessible], expand_pagealigned, PROT_READ | PROT_WRITE);
    if (res != 0)
        err(1, "%s() mprotect failed (%d)", __FUNCTION__, res);

    a->heap_accessible_size += expand_pagealigned;
    a->heap_inaccessible_size -= expand_pagealigned;
    return (int64_t) newly_accessible;
}

char *lucet_alloc_get_heap(struct lucet_alloc const *a)
{
    assert(a);
    return a->heap;
}

uint32_t lucet_alloc_get_heap_len(struct lucet_alloc const *a)
{
    assert(a);
    return a->heap_accessible_size;
}

bool lucet_alloc_mem_in_heap(struct lucet_alloc const *a, void *ptr, size_t len)
{
    assert(a);
    uintptr_t start = (uintptr_t) ptr;
    uintptr_t end   = start + (uintptr_t) len;

    uintptr_t heap_start = (uintptr_t) a->heap;
    uintptr_t heap_end   = heap_start + (uintptr_t) a->heap_accessible_size;

    return (start <= end) && (start >= heap_start) && (start < heap_end) && (end >= heap_start) &&
           (end <= heap_end);
}

bool lucet_alloc_addr_in_heap_guard(struct lucet_alloc const *a, uintptr_t addr)
{
    assert(a);
    uintptr_t const guard_start = (uintptr_t) a->heap + a->heap_accessible_size;
    uintptr_t const guard_end   = (uintptr_t) a->heap + a->limits->heap_address_space_size;
    return (addr >= guard_start) && (addr < guard_end);
}

// Get a pointer to the globals
char *lucet_alloc_get_globals(struct lucet_alloc const *a)
{
    assert(a);
    return a->globals;
}

void lucet_alloc_get_sigstack(struct lucet_alloc const *a, stack_t *ss)
{
    assert(a);
    assert(ss);
    ss->ss_sp    = (void *) a->sigstack;
    ss->ss_flags = 0;
    ss->ss_size  = SIGSTKSZ;
}

const char *lucet_alloc_stat_name(int stat)
{
    switch (stat) {
    case lucet_alloc_ok:
        return "ok";
    case lucet_alloc_spec_over_limits:
        return "spec over limits";
    default:
        return "<invalid>";
    }
}
