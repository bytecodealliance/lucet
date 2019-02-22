#include "greatest.h"
#include "lucet.h"
#include "lucet_libc.h"
#include "test_helpers.h"
#include <assert.h>

#define CURRENT_MEMORY_SANDBOX_PATH "memory_guests/current_memory.so"
#define GROW_MEMORY_SANDBOX_PATH "memory_guests/grow_memory.so"
#define MUSL_ALLOC_SANDBOX_PATH "memory_guests/musl_alloc.so"

TEST current_memory_hostcall(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(CURRENT_MEMORY_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    struct lucet_state state;
    ASSERT_OK(lucet_instance_state(inst, &state));

    uint32_t current_memory_res = LUCET_UNTYPED_RETVAL_TO_U32(state.val.returned);
    // Webassembly module requires 4 pages of memory in import
    ASSERT_EQ(4, current_memory_res);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST grow_memory_hostcall(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(GROW_MEMORY_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    // Guest puts the result of the grow_memory(1) call in heap[0]
    uint8_t *heap            = lucet_instance_heap(inst);
    uint32_t grow_memory_res = ((uint32_t *) heap)[0];
    // Based on the initial memory size in the module, growing by 1 returns prev size 4
    ASSERT_EQ(4, grow_memory_res);
    // Guest then puts the result of the current memory call in heap[4]
    uint32_t current_memory_res = ((uint32_t *) heap)[1];
    ASSERT_EQ(5, current_memory_res);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

static char   output_string[1024];
static char * output_cursor;
static size_t output_cursor_len;
static void   reset_output(void)
{
    memset(output_string, 0, sizeof(output_string));
    output_cursor     = output_string;
    output_cursor_len = sizeof(output_string);
}

static void debug_handler(struct lucet_libc *libc, int32_t fd, const char *buf, size_t len)
{
    assert(fd == 1);
    if (len <= output_cursor_len) {
        memcpy(output_cursor, buf, len);
        output_cursor += len;
        output_cursor_len -= len;
    }
}

TEST musl_alloc(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(MUSL_ALLOC_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_libc lucet_libc;
    reset_output();
    lucet_libc_init(&lucet_libc);
    lucet_libc_set_stdio_handler(&lucet_libc, debug_handler);

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance_with_ctx(region, mod, &lucet_libc, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    ASSERT_STR_EQ("this is a string located in the heap: hello from musl_alloc.c!\n\n",
                  output_string);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

SUITE(memory_suite)
{
    RUN_TEST(current_memory_hostcall);
    RUN_TEST(grow_memory_hostcall);
    RUN_TEST(musl_alloc);
}
