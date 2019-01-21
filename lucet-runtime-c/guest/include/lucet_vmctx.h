
#ifndef LUCET_VMCTX_H
#define LUCET_VMCTX_H

#include <stddef.h>
#include <stdint.h>

// IMPORTANT: This header gives the signatures of the lucet_vmctx functions in
// terms of the wasm32 vm.
// This header should only be included in wasm32 code to be run as a guest (and
// compiled with lucetc)
// Do not include this header in any code to be compiled natively.
//
// The symbols themselves have a different signature in the native abi. lucetc
// translates the wasm32 vm to native abi, so your guest code will call the
// functions correctly.

// The following functions are standardized host calls provided by liblucet.
// Any guest is able to call them.

// Print a debug statement. Only has an effect if the embedding has defined
// a debug handler.
void lucet_vmctx_debug(int32_t fd, const char *buf, size_t len);

// Returns the current number of pages in the guest's heap, in units of wasm
// pages (64k each). Used to implement the wasm `current_memory` instruction.
uint32_t lucet_vmctx_current_memory(void);

// Takes the number of wasm pages to grow the guest heap by. Returns the
// number of pages before the call on success, or -1 on failure. Used to
// implement the wasm `grow_memory` instruction.
int32_t lucet_vmctx_grow_memory(uint32_t additional_pages);

#endif // LUCET_VMCTX_H
