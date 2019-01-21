
#ifndef FEATURES_GUEST_H
#define FEATURES_GUEST_H

#include <stdint.h>

#include "lucet_vmctx.h"

// Morally, the signature of features_get_header is (const char* key, size_t
// key_len, char* value, size_t* value_len). All pointers have been translated
// into offsets into the heap by the caller code, and the size_t key_len has
// been translated to a u32 (size_t is u64 on x86_64) because we are emulating
// the wasm32 runtime where size_t is a u32.
void features_get_header(struct lucet_vmctx *ctx, uint32_t key, uint32_t key_len, uint32_t value,
                         uint32_t value_len);

// Morally, the signature of example_syscall_send is (const char* chunk, size_t
// chunk_len). The chunk pointer has been translated into a heap offset by
// caller, and size_t is restricted to 32 bits to emulate wasm32.
void features_send(struct lucet_vmctx *ctx, uint32_t chunk, uint32_t chunk_len);

#endif // FEATURES_GUEST_H
