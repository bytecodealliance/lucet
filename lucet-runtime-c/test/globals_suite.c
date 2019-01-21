#include <string.h>

#include "inttypes.h"

#include "greatest.h"

#include "../include/lucet.h"

#include "guest_module.h"
#define INTERNAL_MOD_PATH "globals/internal.so"
#define IMPORT_MOD_PATH "globals/import.so"

TEST read_global0(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(INTERNAL_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(inst, "get_global0", 0);

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t global0_val = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(-1, global0_val);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST read_both_globals(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(INTERNAL_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat_run1 = lucet_instance_run(inst, "get_global0", 0);

    ASSERT_ENUM_EQ(lucet_run_ok, stat_run1, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    int64_t global0_val = LUCET_UNTYPED_RETVAL_TO_I64(state->u.ready.untyped_retval);

    ASSERT_EQ(-1, global0_val);

    enum lucet_run_stat const stat_run2 = lucet_instance_run(inst, "get_global1", 0);

    ASSERT_ENUM_EQ(lucet_run_ok, stat_run2, lucet_run_stat_name);

    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    int64_t global1_val = LUCET_UNTYPED_RETVAL_TO_I64(state->u.ready.untyped_retval);
    ASSERT_EQ(420, global1_val);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST mutate_global0(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(INTERNAL_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat_run1 =
        lucet_instance_run(inst, "set_global0", 1, LUCET_VAL_U64(666));
    ASSERT_ENUM_EQ(lucet_run_ok, stat_run1, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    enum lucet_run_stat const stat_run2 = lucet_instance_run(inst, "get_global0", 0);

    ASSERT_ENUM_EQ(lucet_run_ok, stat_run2, lucet_run_stat_name);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    int64_t global0_val = LUCET_UNTYPED_RETVAL_TO_I64(state->u.ready.untyped_retval);

    ASSERT_EQ(666, global0_val);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST reject_import(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(IMPORT_MOD_PATH));
    ASSERT(mod == NULL);

    PASS();
}

SUITE(globals_suite)
{
    RUN_TEST(read_global0);
    RUN_TEST(read_both_globals);
    RUN_TEST(mutate_global0);
    RUN_TEST(reject_import);
}
