#ifndef LUCET_PROBESTACK_H
#define LUCET_PROBESTACK_H

#include <stdint.h>

#include "lucet_export.h"

// lucet_probestack is called from loaded guest programs, therefore
// it must be exported from the library. However, it is not part of the
// library's public API. lucet_probextack has a special ABI (not the same as an
// ordinary function call) and should not be called by anything other than
// generated guest code.
void lucet_probestack(void) EXPORTED;

// When a page fault is caused from inside lucet_probestack, that means the caller
// function is checking whether each page of the stack frame it is about to
// expand into is valid memory (probestack is called as part of the function
// prelude). The lucet_trap mechanism needs to determine whether an instruction
// falls inside lucet_probestack. This variable is defined equal to the size of
// the lucet_probestack text.
extern const uint32_t lucet_probestack_size;

#endif // LUCET_PROBESTACK_H
