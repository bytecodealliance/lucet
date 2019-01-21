#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"
#include "lucet_libc.h"
#include <assert.h>

#define CURRENT_MEMORY_SANDBOX_PATH "memory_guests/current_memory.so"
#define GROW_MEMORY_SANDBOX_PATH "memory_guests/grow_memory.so"
#define MUSL_ALLOC_SANDBOX_PATH "memory_guests/musl_alloc.so"

TEST current_memory_hostcall(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CURRENT_MEMORY_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    uint32_t current_memory_res = LUCET_UNTYPED_RETVAL_TO_U32(state->u.ready.untyped_retval);
    // Webassembly module requires 4 pages of memory in import
    ASSERT_EQ(4, current_memory_res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST grow_memory_hostcall(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(GROW_MEMORY_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Guest puts the result of the grow_memory(1) call in heap[0]
    char *   heap            = lucet_instance_get_heap(inst);
    uint32_t grow_memory_res = ((uint32_t *) heap)[0];
    // Based on the current liblucet settings, growing by 1 returns prev size 4
    ASSERT_EQ(4, grow_memory_res);
    // Guest then puts the result of the current memory call in heap[4]
    uint32_t current_memory_res = ((uint32_t *) heap)[1];
    ASSERT_EQ(5, current_memory_res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

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
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(MUSL_ALLOC_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_libc lucet_libc;
    reset_output();
    lucet_libc_init(&lucet_libc);
    lucet_libc_set_stdio_handler(&lucet_libc, debug_handler);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, &lucet_libc);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    ASSERT_STR_EQ("this is a string located in the heap: hello from musl_alloc.c!\n\n",
                  output_string);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

SUITE(memory_suite)
{
    RUN_TEST(current_memory_hostcall);
    RUN_TEST(grow_memory_hostcall);
    RUN_TEST(musl_alloc);
}
