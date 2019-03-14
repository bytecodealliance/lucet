#ifndef common_H
#define common_H

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "lucet.h"

#define LUCET_HEAP(CTX) lucet_vmctx_get_heap(CTX)

#define LUCET_CURRENT_HEAP_SIZE(CTX) (lucet_vmctx_current_memory(CTX) * LUCET_WASM_PAGE_SIZE)

#define TRAP __asm__ __volatile__("ud2")

#define TRAPIF(C) \
    if (C)        \
    TRAP

#endif
