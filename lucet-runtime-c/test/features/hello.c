#include <assert.h>
#include <stdio.h>
#include <string.h>

// The lucet_vmctx_exit and lucet_vmctx_debug hostcall funcs are declared in
// lucet_vmctx.h in their native abi signatures.
#include "lucet_vmctx.h"

// Remaining hostcall funcs are coming from features_guest.h. These call the
// implementions in session.c/h.

#include "features_guest.h"

// We need the repr of lucet_vmctx and lucet_alloc_heap_spec
#include "../helpers.h"
#include "../include/lucet_constants.h"
#include "../src/lucet_module_private.h"
#include "../src/lucet_vmctx_private.h"

DEFINE_DEFAULT_GLOBAL_SPEC;

struct lucet_alloc_heap_spec lucet_heap_spec = {
    .initial_size = 64 * 1024,
    .max_size     = 64 * 1024,
    .guard_size   = 4 * 1024 * 1024,
};

// All data passed to syscalls has to be located on the heap, for security
// reasons. (This code is emulating code that was compiled through webassembly,
// so the heap is the only memory that the code should be able to access). So,
// we allocate slices of the heap (pretty arbitrarily) to contain the various
// chunks of data that have to be passed off to syscalls.
#define HEAP_POS_HELLO 0
#define HEAP_POS_KEY 128
#define HEAP_POS_VAL 192
#define HEAP_POS_VAL_LEN 256
#define HEAP_POS_OUTPUT 320
#define HEAP_POS_SYSCALL_ARGS 384

// liblucet expects the WASM .so it loads to supply WASM data segment
// iniitialization info via the symbols defined below. liblucet uses this info
// to copy initial data into linear memory when a module is instantiated.
//
// Presently liblucet is using a format implicitly defined in lucetc, which is
// mimicked below.
const char wasm_data_segments[] =
    "\x00\x00\x00\x00" // 0: memdix
    "\x00\x00\x00\x00" // 4: offset
    "\x1D\x00\x00\x00" // 8: length
    "this should be overwritten!!\x00"
    // ^ 12: data stored at heap pos 0
    "\x00\x00\x00\x00" // 41: pad to %8
    "\x00\x00\x00"

    "\x00\x00\x00\x00" // 48: memdix
    "\x1D\x00\x00\x00" // 52: offset
    "\x23\x00\x00\x00" // 56: length
    "hello again from sandbox_native.c!\x00"
    // ^ 60: data stored at heap pos 48
    "\x00" // 95: pad to %8

    "\x00\x00\x00\x00" // 96: memdix
    "\x00\x00\x00\x00" // 100: offset (overwrites first segment)
    "\x1D\x00\x00\x00" // 104: length
    "hello from sandbox_native.c!\x00"
    // ^ 108: data stored at heap pos 0
    "\x00\x00\x00\x00" // 149: pad to %8
    "\x00\x00";        // N.b. C will append a null byte
const uint32_t wasm_data_segments_len = sizeof(wasm_data_segments);

// The sandbox is run by calling this function
void guest_func_main(struct lucet_vmctx *ctx)
{
    char * heap       = (char *) ctx;
    size_t heap_pages = lucet_vmctx_current_memory(ctx);
    if ((heap_pages * LUCET_WASM_PAGE_SIZE) < (HEAP_POS_SYSCALL_ARGS + 6 * sizeof(uint64_t))) {
        lucet_vmctx_terminate(ctx, NULL);
    }

    // Send output - the HEAP_POS_HELLO addr has the `hello`s supplied in the
    // WASM data segment definition
    features_send(ctx, HEAP_POS_HELLO, 29);      // see wasm_data_segment
    features_send(ctx, HEAP_POS_HELLO + 29, 35); // see wasm_data_segment

    // Put the key into the heap
    char  key[]     = "X-Sandbox";
    char *key_reloc = &heap[HEAP_POS_KEY];
    memcpy(key_reloc, key, sizeof(key));

    // Val is allocated as a slice of the heap, as well as the val_len ptr.
    // These will be filled in by the _get_header syscall.
    char *  val_reloc     = &heap[HEAP_POS_VAL];
    size_t *val_len_reloc = (size_t *) &heap[HEAP_POS_VAL_LEN];
    *val_len_reloc        = 64; // The slice is 64 bytes max. After the call, this value will be
    features_get_header(ctx, HEAP_POS_KEY, sizeof(key) - 1, HEAP_POS_VAL, HEAP_POS_VAL_LEN);

    // output is allocated as a slice of the heap, we will snprintf the return
    // value from
    char * output_reloc = &heap[HEAP_POS_OUTPUT];
    size_t out_len;
    if (*val_len_reloc > 0) {
        out_len = snprintf(output_reloc, 128, "got sandbox key: %s\n", val_reloc);
    } else {
        out_len = snprintf(output_reloc, 128, "sandbox key not found :(\n");
    }

    features_send(ctx, HEAP_POS_OUTPUT, out_len);

    // Test exit codes
    if (val_reloc[0] == '3') {
        out_len = snprintf(output_reloc, 128, "going to exit with code -1\n");
        features_send(ctx, HEAP_POS_OUTPUT, out_len);
        lucet_vmctx_terminate(ctx, (void *) -1);
    }

    // Try to get function address from ID
    const char *func;
    func = lucet_vmctx_get_func_from_id(ctx, 0, 1);
    assert(func == NULL);
    func = lucet_vmctx_get_func_from_id(ctx, 1, 0);
    assert(func == NULL);
    func = lucet_vmctx_get_func_from_id(ctx, 0, 0);
    assert(func != NULL);
}

struct lucet_table_element guest_table_0 = { .element_type = (uint64_t) 0x0,
                                             .ref =
                                                 (uint64_t)(uintptr_t)(void *) &guest_func_main };

uint64_t guest_table_0_len = sizeof guest_table_0;
