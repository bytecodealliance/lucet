#include <inttypes.h>

#include "../helpers.h"
#include "lucet_vmctx.h"

DEFINE_DEFAULT_HEAP_SPEC;
DEFINE_DEFAULT_GLOBAL_SPEC;
DEFINE_DEFAULT_DATA_SEGMENTS;
DEFINE_DEFAULT_SPARSE_PAGE_DATA;

// forward decl
void guest_func_illegal_instr(void *);
void guest_func_oob(struct lucet_vmctx *);

const uint32_t lucet_trap_manifest_len = 2;

uint32_t guest_traps_illegal_instr[] = {
    8, // offset into guest_func_illegal_instr of trapping instruction
    4, // trap code (lucet_trapcode_bad_signature)
};

uint32_t guest_traps_oob[] = {
    38, // offset into guest_func_oob of trapping instruction
    1,  // trap code (lucet_trapcode_heap_oob)
};

// Note:
// Manually creating a trap manifest structure like this is almost certain to
// fragile at best and flakey at worst. If it ends up causing issues, the right
// answer is probably to rewrite this in assembly.
uint64_t lucet_trap_manifest[] = {
    // -- trap manifest --
    (uint64_t)((uintptr_t) &guest_func_illegal_instr), // pointer to function
    11,                                                // length of guest_func_illegal_instr
    (uint64_t)((uintptr_t) guest_traps_illegal_instr), // pointer to table
    1,                                       // length of guest_func_illegal_instr's trap table
    (uint64_t)((uintptr_t) &guest_func_oob), // pointer to function
    42,                                      // length of guest_func_oob
    (uint64_t)((uintptr_t) guest_traps_oob), // pointer to table
    1,                                       // length of guest_func_oob's trap table
};

void guest_func_illegal_instr(void *ctx)
{
    asm("ud2");
}

void guest_func_oob(struct lucet_vmctx *ctx)
{
    char *heap = lucet_vmctx_get_heap(ctx);
    // According to lucet_heap_spec above, the initial and max size of this
    // guest's heap is 64k, with a 4m guard size. So, we'll access memory
    // that is just beyond the 64k limit.
    heap[64 * 1024 + 1] = '\0';
}

void guest_func_fatal(void *ctx)
{
    char *heap = lucet_vmctx_get_heap(ctx);
    // According to lucet_heap_spec above, the initial and max size of this
    // guest's heap is 64k, with a 4m guard size. After the guard there will
    // be no globals, and then a stack (not specified above).
    // Empirically, at the 4m + 128k point, memory is unmapped. This may change
    // as the library, test configuration, linker, phase of moon, etc change,
    // but for now it works.
    heap[(4 * 1024 * 1024) + (128 * 1024)] = '\0';
}

extern char *guest_recoverable_get_ptr(void);

void guest_func_recoverable_fatal(void *ctx)
{
    char *ptr = guest_recoverable_get_ptr();
    *ptr      = '\0';
}

void guest_func_infinite_loop(void *ctx)
{
    for (;;)
        ;
}

int guest_func_onetwothree(void *ctx)
{
    return 123;
}
