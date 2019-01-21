#ifndef LUCET_ALLOC_PRIVATE_H
#define LUCET_ALLOC_PRIVATE_H

#include <signal.h>
#include <stdbool.h>
#include <stddef.h>

#include "lucet_alloc.h"
#include "lucet_decls.h"
#include "lucet_globals_private.h"

struct lucet_alloc_region;
struct lucet_alloc_runtime_spec;

struct lucet_alloc {
    // a `lucet_alloc` struct manages a contigious chunk of virtual memory. The
    // size of this memory is determined by the `lucet_alloc_limits` struct given
    // at creation.
    // the first part of this memory, pointed to by `start`, is always backed by
    // real memory.  It is used to store the lucet_instance structure.
    char *start;

    // The next part of memory contains the heap (backed by real memory
    // according to the heap_spec, and grow memory calls) and the guard pages
    // for the heap (these pages trigger a sigsegv when accessed).
    // Accessible from `_allocate_runtime` until `_free_runtime`.
    char * heap;
    size_t heap_accessible_size;
    size_t heap_inaccessible_size;

    // After the heap comes the stack. Because the stack grows downwards,
    // we get the added safety of ensuring that stack overflows go into
    // the guard pages, if the limits specify guard pages. Stack is always
    // the size given by `limits->stack-size`.
    // Accessible from `_allocate_runtime` until `_free_runtime`.
    char *stack;

    // After the stack there is a single guard page.

    // After the stack guard page, there is an array for the webassembly
    // globals.
    // Accessible from `_allocate_runtime` until `_free_runtime`.
    char *globals;

    // After the globals, there is a separate stack for the signal handler
    // to run in. This allows the signal handler to run in situations where
    // the normal stack has grown into the guard page.
    char *sigstack;

    // Limits of the memory. Set on creation, does not change throughout the
    // lifetime of the structure.
    struct lucet_alloc_limits const *limits;

    // Spec for the runtime of the current instance. This is valid from an
    // _allocate_runtime until _free_runtime.
    struct lucet_alloc_runtime_spec const *spec;

    // The parent of this allocation
    struct lucet_alloc_region *region;
};

struct lucet_alloc_heap_spec;

// Modules have flexibility about how much of the heap & globals region they
// use. This structure makes sure we have access to that info:
struct lucet_alloc_runtime_spec {
    struct lucet_alloc_heap_spec const *heap;
    struct lucet_globals_spec const *   globals;
};

// Each module ships with a specification of the heap model used to compile it,
// and the initial and maximum sizes declared by the webassembly module.
// The reserved size and the guard size, together, must fit in the
// heap_address_space_size given in the pool limits.
// The initial size and the max size (if given) must fit in the heap_memory_size
// given in the pool limits.
struct lucet_alloc_heap_spec {
    /**
     * Reserved size: a region of the heap that is addressable, but only a
     * subset of it is accessible. Specified in bytes. Must be divisible by 4k
     * (host page).
     */
    uint64_t reserved_size;
    /**
     * Guard size: a region of the heap that is addressable, but never
     * accessible. Specified in bytes. Must be divisible by 4k (host page).
     */
    uint64_t guard_size;
    /**
     * Initial size: the amount of the heap that is accessible on
     * initialization. Specified in bytes. Must be divisible by 64k (webassembly
     * page).
     * Must be less or equal to the reserved size.
     */
    uint64_t initial_size;
    /**
     * Maximum size: the maximum amount of the heap that the program will
     * request, iff max_size_valid is true. This comes right from the
     * WebAssembly program's memory definition.
     */
    uint64_t max_size;
    /**
     * Max size valid: set to 1 when max size is valid, 0 when it is not. Why
     * is this a u64 instead of a bool? I didn't want to deal with serialization
     * nonsense when we're going to get a new serializer soon anyway.
     */
    uint64_t max_size_valid;
};

// Abstract region. The lucet_alloc implementations (only one compiled into the
// lib) get to define what this means, private to their object file only.
struct lucet_alloc_region;

// Create (and destroy) an allocation region.
struct lucet_alloc_region *lucet_alloc_create_region(int                              num_entries,
                                                     struct lucet_alloc_limits const *limits);

void lucet_alloc_free_region(struct lucet_alloc_region *region);

// Get an allocation from a region. Its lifetime is linked to that of the
// region. Returns NULL if index is out-of-bounds.
struct lucet_alloc *lucet_alloc_region_get_alloc(struct lucet_alloc_region const *region,
                                                 int                              index);

// Get a pointer to the lucet_instance stored in the memory managed by the alloc
// struct.
struct lucet_instance *lucet_alloc_get_instance(struct lucet_alloc const *);

// Get a pointer to the starting point of the stack.
char *lucet_alloc_get_stack_top(struct lucet_alloc const *);

enum lucet_alloc_stat {
    lucet_alloc_ok,
    lucet_alloc_spec_over_limits,
};
// The _allocate_instance function leaves the lucet_instance struct mapped to
// valid alloc, but none of the rest of the arena. The _allocate_runtime function
// maps the heap, stack, and globals to valid memory.
enum lucet_alloc_stat lucet_alloc_allocate_runtime(struct lucet_alloc *,
                                                   struct lucet_alloc_runtime_spec const *spec);

// Reset a runtime to clear any state in the heap. Precondition: runtime already
// allocated.
void lucet_alloc_reset_runtime(struct lucet_alloc *, struct lucet_module const *mod);

// The _free_runtime function mprotects the allocated heap, stack, and globals
// back to PROT_NONE, effectively freeing the alloc associated with it, while
// keeping the virtual address range intact.
void lucet_alloc_free_runtime(struct lucet_alloc *a);

// Get a pointer to the heap
char *lucet_alloc_get_heap(struct lucet_alloc const *);

// Get the current heap length, in bytes.
uint32_t lucet_alloc_get_heap_len(struct lucet_alloc const *);

// Expand the heap by some number of bytes. Return the offset in the heap that
// the new space starts at. Positive return values, on success will always be
// <UINT32_MAX. Return of -1 indicates failure.
int64_t lucet_alloc_expand_heap(struct lucet_alloc *a, uint32_t amount);

// Check that memory region is inside the heap
bool lucet_alloc_mem_in_heap(struct lucet_alloc const *, void *ptr, size_t len);
// Check that an address is inside the heap guard
bool lucet_alloc_addr_in_heap_guard(struct lucet_alloc const *, uintptr_t addr);

// Offset between the address of the `lucet_instance` struct and the start of the
// heap.
extern const size_t lucet_alloc_instance_heap_offset;

// Get a pointer to the globals
char *lucet_alloc_get_globals(struct lucet_alloc const *);

// Initialize a structure describing the signal stack
void lucet_alloc_get_sigstack(struct lucet_alloc const *, stack_t *ss);

const char *lucet_alloc_stat_name(int lucet_alloc_stat);

#endif // LUCET_ALLOC_PRIVATE_H
