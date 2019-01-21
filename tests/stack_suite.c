#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"

#define LOCALS64_SANDBOX_PATH "stack_guests/locals_64.so"
#define LOCALS_1PAGE_SANDBOX_PATH "stack_guests/locals_1page.so"
#define LOCALS_MULTIPAGE_SANDBOX_PATH "stack_guests/locals_multipage.so"

#define STACK_PER_RECURSION 0xe8

TEST expect_ok(const char *path, int recursion_depth)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(path));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "localpalooza", 1, LUCET_VAL_C_INT(recursion_depth));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST expect_stack_overflow(const char *path, int recursion_depth)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(path));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "localpalooza", 1, LUCET_VAL_C_INT(recursion_depth));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    // We should get a nonfatal trap due to the stack overflow.
    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);
    ASSERT_EQ(false, state->u.fault.fatal);
    ASSERT_EQ(lucet_trapcode_stack_overflow, state->u.fault.trapcode.code);
    ASSERT_EQ(0, state->u.fault.trapcode.tag);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST expect_stack_overflow_probestack(const char *path, int recursion_depth)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(path));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "localpalooza", 1, LUCET_VAL_C_INT(recursion_depth));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    // We should get a nonfatal trap due to the stack overflow.
    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);
    ASSERT_EQ(false, state->u.fault.fatal);
    ASSERT_EQ(lucet_trapcode_stack_overflow, state->u.fault.trapcode.code);
    // When liblucet catches probestack, it puts this special tag in the trapcode.
    ASSERT_EQ(UINT16_MAX, state->u.fault.trapcode.tag);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

SUITE(stack_suite)
{
    // The test with 64 locals should take up 252 bytes per stack frame. Along
    // with the overhead for the sandbox, that means it should overflow on the
    // 455th recursion.  The trap table knows about all of the instructions in
    // the function that manipulate the stack, so the catch mechanism for this
    // is the usual one.
    RUN_TESTp(expect_ok, LOCALS64_SANDBOX_PATH, 1);
    RUN_TESTp(expect_ok, LOCALS64_SANDBOX_PATH, 2);
    RUN_TESTp(expect_ok, LOCALS64_SANDBOX_PATH, 454);
    RUN_TESTp(expect_stack_overflow, LOCALS64_SANDBOX_PATH, 455);

    // This test has about 1 page worth of locals - just enough for Cretonne to
    // use probestack to grow the stack. The 31st recursion should cause a stack
    // overflow.
    RUN_TESTp(expect_ok, LOCALS_1PAGE_SANDBOX_PATH, 1);
    RUN_TESTp(expect_ok, LOCALS_1PAGE_SANDBOX_PATH, 2);
    RUN_TESTp(expect_ok, LOCALS_1PAGE_SANDBOX_PATH, 30);
    RUN_TESTp(expect_stack_overflow_probestack, LOCALS_1PAGE_SANDBOX_PATH, 31);

    // This test has 5000 locals - over 4 pages worth. Cretonne will use
    // probestack here as well. The 6th recursion should cause a stack overflow.
    RUN_TESTp(expect_ok, LOCALS_MULTIPAGE_SANDBOX_PATH, 1);
    RUN_TESTp(expect_ok, LOCALS_MULTIPAGE_SANDBOX_PATH, 2);
    RUN_TESTp(expect_ok, LOCALS_MULTIPAGE_SANDBOX_PATH, 5);
    RUN_TESTp(expect_stack_overflow_probestack, LOCALS_MULTIPAGE_SANDBOX_PATH, 6);
}
