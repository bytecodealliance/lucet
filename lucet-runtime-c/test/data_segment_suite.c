#include <string.h>

#include "inttypes.h"

#include "greatest.h"

#include "../include/lucet.h"
#include "../src/lucet_instance_private.h"
#include "../src/lucet_module_private.h"
#include "guest_module.h"

#define VALID_SANDBOX_PATH "data_segment/valid_data_seg.so"
#define NO_DATA_SEG_SANDBOX_PATH "data_segment/no_data_seg.so"
#define NO_DATA_SEG_LEN_SANDBOX_PATH "data_segment/no_data_seg_len.so"
#define NO_DATA_SEG_INFO_SANDBOX_PATH "data_segment/no_data_seg_info.so"
#define OVERSIZE_DATA_SEGS_SANDBOX_PATH "data_segment/oversize_data_segs.so"
#define OVERSIZE_DATA_SEG_SANDBOX_PATH "data_segment/oversize_data_seg.so"

TEST test_valid_segments(void)
{
    const char valid_segments[] =
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

    uint32_t valid_segments_len = sizeof(valid_segments);

    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(VALID_SANDBOX_PATH));

    ASSERT(mod->data_segment.len == valid_segments_len);
    ASSERT(memcmp(mod->data_segment.segments, valid_segments, valid_segments_len) == 0);

    lucet_module_unload(mod);

    PASS();
}

TEST test_invalid_syms(void)
{
    struct lucet_module *mod;

    // When the WASM module doesn't specify WASM data segment initializers,
    // the compiler omits the corresponding symbols and lucet_load sets the
    // corresponding lucet_module values to zero.

    // If the compiler output specifies the WASM data segment but no
    // length, this is an error.
    mod = lucet_module_load(guest_module_path(NO_DATA_SEG_SANDBOX_PATH));
    ASSERT(mod == NULL);

    // If the compiler output specifies the WASM data segment length but no
    // segment, this is an error.
    mod = lucet_module_load(guest_module_path(NO_DATA_SEG_LEN_SANDBOX_PATH));
    ASSERT(mod == NULL);

    // If the compiler output specifies no WASM data segment info, this is an
    // error.
    mod = lucet_module_load(guest_module_path(NO_DATA_SEG_INFO_SANDBOX_PATH));
    ASSERT(mod == NULL);
    PASS();
}

TEST instantiate_valid_data_segs_uses_initdata_or_fatal(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(VALID_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // The test data initializers result in two strings getting copied into
    // linear memory; see guest test file for details
    const char *first_message = "hello from sandbox_native.c!";
    ASSERT(strcmp((char *) inst->alloc->heap, first_message) == 0);

    const char *second_message = "hello again from sandbox_native.c!";
    ASSERT(strcmp((char *) (inst->alloc->heap + strlen(first_message) + 1), second_message) == 0);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_oversize_data_segs(void)
{
    // Tests that a total length of all data segment initializers that exceeds
    // the available linear memory causes instantiation to fail

    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(OVERSIZE_DATA_SEGS_SANDBOX_PATH));
    ASSERT(mod == NULL);

    PASS();
}

TEST test_oversize_data_seg(void)
{
    // Tests that a data segment initializer that specifies a length that
    // execeeds the available linear memory causes instantiation to fail

    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(OVERSIZE_DATA_SEG_SANDBOX_PATH));
    ASSERT(mod == NULL);

    PASS();
}

SUITE(data_seg_init_suite)
{
    RUN_TEST(test_valid_segments);
    RUN_TEST(test_invalid_syms);
    RUN_TEST(instantiate_valid_data_segs_uses_initdata_or_fatal);
    RUN_TEST(test_oversize_data_segs);
    RUN_TEST(test_oversize_data_seg);
}
