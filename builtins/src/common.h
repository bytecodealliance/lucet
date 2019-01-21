#ifndef common_H
#define common_H

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "lucet_constants.h"
#include "lucet_vmctx.h"

#define LUCET_HEAP(CTX) lucet_vmctx_get_heap(CTX)

#ifdef STATIC_HEAP_SIZE
#define LUCET_CURRENT_HEAP_SIZE(CTX) STATIC_HEAP_SIZE
#else
#define LUCET_CURRENT_HEAP_SIZE(CTX) (lucet_vmctx_current_memory(CTX) * LUCET_WASM_PAGE_SIZE)
#endif

#define TRAP __asm__ __volatile__("ud2")

#define TRAPIF(C) \
    if (C)        \
    TRAP

#endif
