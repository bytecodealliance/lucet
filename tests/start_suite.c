#include <assert.h>

#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"

#define GLOBAL_INIT_SANDBOX_PATH "start_guests/global_init.so"
#define START_AND_CALL_SANDBOX_PATH "start_guests/start_and_call.so"
#define NO_START_SANDBOX_PATH "start_guests/no_start.so"

TEST global_init(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(GLOBAL_INIT_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const start_stat = lucet_instance_run_start(inst);
    ASSERT_ENUM_EQ(lucet_run_ok, start_stat, lucet_run_stat_name);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);
    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Now the globals should be:
    // $flossie = 17
    // and heap should be:
    // [0] = 17

    char *heap = lucet_instance_get_heap(inst);

    uint32_t global_flossie_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(17, global_flossie_read);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST start_and_call(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(START_AND_CALL_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const start_stat = lucet_instance_run_start(inst);
    ASSERT_ENUM_EQ(lucet_run_ok, start_stat, lucet_run_stat_name);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);
    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Now the globals should be:
    // $flossie = 17
    // and heap should be:
    // [0] = 17

    char *heap = lucet_instance_get_heap(inst);

    uint32_t global_flossie_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(17, global_flossie_read);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST no_start(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(NO_START_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // this should not do anything
    enum lucet_run_stat const start_stat = lucet_instance_run_start(inst);
    ASSERT_ENUM_EQ(lucet_run_symbol_not_found, start_stat, lucet_run_stat_name);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);
    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Now the globals should be:
    // $flossie = 17
    // and heap should be:
    // [0] = 17

    char *heap = lucet_instance_get_heap(inst);

    uint32_t global_flossie_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(17, global_flossie_read);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

SUITE(start_suite)
{
    RUN_TEST(global_init);
    RUN_TEST(start_and_call);
    RUN_TEST(no_start);
}
