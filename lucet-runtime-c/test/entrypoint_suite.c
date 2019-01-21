#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"

#define CALCULATOR_MOD_PATH "entrypoint/calculator.so"

TEST test_calc_add_2(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_2", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);
    ASSERT_EQ(123 + 456, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_10(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_10", 10, LUCET_VAL_U64(1), LUCET_VAL_U64(2), LUCET_VAL_U64(3),
                           LUCET_VAL_U64(4), LUCET_VAL_U64(5), LUCET_VAL_U64(6), LUCET_VAL_U64(7),
                           LUCET_VAL_U64(8), LUCET_VAL_U64(9), LUCET_VAL_U64(10));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_mul_2(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "mul_2", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(123 * 456, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_then_mul(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_2", 2, LUCET_VAL_U64(111), LUCET_VAL_U64(222));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(111 + 222, res);

    enum lucet_run_stat const stat2 =
        lucet_instance_run(inst, "mul_2", 2, LUCET_VAL_U64(333), LUCET_VAL_U64(444));
    ASSERT_ENUM_EQ(lucet_run_ok, stat2, lucet_run_stat_name);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res2 = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);
    ASSERT_EQ(333 * 444, res2);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_invalid_entrypoint(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "invalid", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));
    ASSERT_ENUM_EQ(lucet_run_symbol_not_found, stat, lucet_run_stat_name);

    const struct lucet_state *state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);
    ASSERT_EQ(0, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_f32_2(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_f32_2", 2, LUCET_VAL_F32(-6.9), LUCET_VAL_F32(4.2));

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    float res = LUCET_UNTYPED_RETVAL_TO_F32(state->u.ready.untyped_retval);
    ASSERT_EQ(-6.9f + 4.2f, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_f64_2(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_f64_2", 2, LUCET_VAL_F64(-6.9), LUCET_VAL_F64(4.2));

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    double res = LUCET_UNTYPED_RETVAL_TO_F64(state->u.ready.untyped_retval);
    ASSERT_EQ(-6.9 + 4.2, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_f32_10(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "add_f32_10", 10, LUCET_VAL_F32(0.1), LUCET_VAL_F32(0.2), LUCET_VAL_F32(0.3),
        LUCET_VAL_F32(0.4), LUCET_VAL_F32(0.5), LUCET_VAL_F32(0.6), LUCET_VAL_F32(0.7),
        LUCET_VAL_F32(0.8), LUCET_VAL_F32(0.9), LUCET_VAL_F32(1.0));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    float res = LUCET_UNTYPED_RETVAL_TO_F32(state->u.ready.untyped_retval);

    ASSERT_EQ(0.1f + 0.2f + 0.3f + 0.4f + 0.5f + 0.6f + 0.7f + 0.8f + 0.9f + 1.0f, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_f64_10(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "add_f64_10", 10, LUCET_VAL_F64(0.1), LUCET_VAL_F64(0.2), LUCET_VAL_F64(0.3),
        LUCET_VAL_F64(0.4), LUCET_VAL_F64(0.5), LUCET_VAL_F64(0.6), LUCET_VAL_F64(0.7),
        LUCET_VAL_F64(0.8), LUCET_VAL_F64(0.9), LUCET_VAL_F64(1.0));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    double res = LUCET_UNTYPED_RETVAL_TO_F64(state->u.ready.untyped_retval);

    ASSERT_EQ(0.1 + 0.2 + 0.3 + 0.4 + 0.5 + 0.6 + 0.7 + 0.8 + 0.9 + 1.0, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_mixed_20(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_MOD_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "add_mixed_20", 20, LUCET_VAL_F64(-1.1), LUCET_VAL_U8(1), LUCET_VAL_F32(2.1),
        LUCET_VAL_F64(3.1), LUCET_VAL_U16(4), LUCET_VAL_F32(5.1), LUCET_VAL_F64(6.1),
        LUCET_VAL_U32(7), LUCET_VAL_F32(8.1), LUCET_VAL_F64(9.1), LUCET_VAL_BOOL(1),
        LUCET_VAL_F32(11.1), LUCET_VAL_F64(12.1), LUCET_VAL_C_INT(13), LUCET_VAL_F32(14.1),
        LUCET_VAL_F64(15.1), LUCET_VAL_C_LONGLONG(16), LUCET_VAL_F32(17.1), LUCET_VAL_F64(18.1),
        LUCET_VAL_C_LONGLONG(19));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    double res = LUCET_UNTYPED_RETVAL_TO_F64(state->u.ready.untyped_retval);

    ASSERT_EQ((double) -1.1 + (double) 1U + (double) 2.1f + (double) 3.1 + (double) 4U +
                  (double) 5.1f + (double) 6.1 + (double) 7U + (double) 8.1f + (double) 9.1 +
                  (double) 1 + (double) 11.1f + (double) 12.1 + (double) 13 + (double) 14.1f +
                  (double) 15.1 + (double) 16LL + (double) 17.1f + (double) 18.1 + (double) 19LL,
              res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

SUITE(entrypoint_suite)
{
    RUN_TEST(test_calc_add_2);
    RUN_TEST(test_calc_add_10);
    RUN_TEST(test_calc_mul_2);
    RUN_TEST(test_calc_add_then_mul);
    RUN_TEST(test_calc_invalid_entrypoint);
    RUN_TEST(test_calc_add_f32_2);
    RUN_TEST(test_calc_add_f64_2);
    RUN_TEST(test_calc_add_f32_10);
    RUN_TEST(test_calc_add_f64_10);
    RUN_TEST(test_calc_add_mixed_20);
}
