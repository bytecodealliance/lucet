#ifndef TEST_HELPERS_H
#define TEST_HELPERS_H

#include "../src/lucet_alloc_private.h"

// Most tests don't exercise the functionality of heap size specialization,
// data segments, globals, etc. These macros are provided so for those that do
// not, we can reduce the size of the tests and minimize the number of places
// we have to change when the format changes.

#define DEFAULT_HEAP_SPEC                                                      \
    {                                                                          \
        .reserved_size = 4 * 1024 * 1024, .guard_size = 4 * 1024 * 1024,       \
        .initial_size = 64 * 1024, .max_size = 64 * 1024, .max_size_valid = 1, \
    }
#define DEFINE_DEFAULT_HEAP_SPEC struct lucet_alloc_heap_spec lucet_heap_spec = DEFAULT_HEAP_SPEC

#define DEFAULT_GLOBAL_SPEC \
    {                       \
        .num_globals = 0,   \
    }
#define DEFINE_DEFAULT_GLOBAL_SPEC \
    struct lucet_globals_spec lucet_globals_spec = DEFAULT_GLOBAL_SPEC

#define DEFINE_DEFAULT_DATA_SEGMENTS                        \
    const char     wasm_data_segments[]   = "\00\00\00\00"; \
    const uint32_t wasm_data_segments_len = sizeof(wasm_data_segments)

#endif
