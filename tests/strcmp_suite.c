#include <assert.h>
#include <stdio.h>

#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"

#define FAULT_MOD_PATH "strcmp_guests/fault_guest.so"

void hostcall_host_fault(void *vmctx)
{
    (void) vmctx;
    char *oob = (char *) -1;
    *oob      = 'x';
}

TEST run_strcmp(const char *mod_path, const char *s1, const char *s2, int *res,
                struct lucet_state *exit_state)
{
    // Test helper function. Runs strcmp in the guest.
    ASSERTm("precondition mod_path", mod_path);
    ASSERTm("precondition s1", s1);
    ASSERTm("precondition s2", s2);
    ASSERTm("precondition res", res);
    ASSERTm("precondition exit_state", exit_state);

    const size_t res_size = sizeof(int64_t);
    const size_t s1_size  = strlen(s1) + 1;
    const size_t s2_size  = strlen(s2) + 1;
    ASSERTm("precondition sizes", (res_size + s1_size + s2_size) < LUCET_WASM_PAGE_SIZE);

    // Pool size reduced from 1000 to 10 because we sometimes can't to get
    // enough virtual memory when running inside docker or on CI.
    struct lucet_pool *pool = lucet_pool_create(10, NULL);
    ASSERTm("failed to create pool", pool != NULL);
    struct lucet_module *mod = lucet_module_load(guest_module_path(mod_path));
    ASSERTm("failed to load module", mod != NULL);

    struct lucet_instance *instance;
    instance = lucet_instance_create(pool, mod, NULL);
    ASSERTm("lucet_instance_create returned NULL", instance != NULL);

    char *  heap          = lucet_instance_get_heap(instance);
    int32_t newpage_start = lucet_instance_grow_memory(instance, 1);
    ASSERTm("unable to grow memory for args", newpage_start > 0);

    const guest_ptr_t res_ptr = newpage_start * LUCET_WASM_PAGE_SIZE;
    const guest_ptr_t s1_ptr  = res_ptr + res_size;
    const guest_ptr_t s2_ptr  = s1_ptr + s1_size;
    memcpy(&heap[s1_ptr], s1, s1_size);
    memcpy(&heap[s2_ptr], s2, s2_size);

    enum lucet_run_stat const stat =
        lucet_instance_run(instance, "run_strcmp", 3, LUCET_VAL_GUEST_PTR(s1_ptr),
                           LUCET_VAL_GUEST_PTR(s2_ptr), LUCET_VAL_GUEST_PTR(res_ptr));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    // Copy out state as of program termination
    const struct lucet_state *state;
    state = lucet_instance_get_state(instance);
    memcpy(exit_state, state, sizeof(struct lucet_state));

    if (state->tag == lucet_state_ready) {
        *res = LUCET_UNTYPED_RETVAL_TO_C_INT(state->u.ready.untyped_retval);
    } else {
        // Make sure the result is obviously wrong
        *res = 666;
    }

    lucet_instance_release(instance);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);
    PASS();
}

TEST strcmp_compare(const char *input1, const char *input2)
{
    // Test compares native strcmp to strcmp running as a guest.
    int                res;
    struct lucet_state end_state;

    CHECK_CALL(run_strcmp(FAULT_MOD_PATH, input1, input2, &res, &end_state));

    ASSERT_ENUM_EQ(lucet_state_ready, end_state.tag, lucet_state_name);

    ASSERT_EQ_FMT(strcmp(input1, input2), res, "%d");

    PASS();
}

TEST wasm_fault_test(void)
{
    struct lucet_pool *pool = lucet_pool_create(10, NULL);
    ASSERTm("failed to create pool", pool != NULL);
    struct lucet_module *mod = lucet_module_load(guest_module_path(FAULT_MOD_PATH));
    ASSERTm("failed to load module", mod != NULL);

    struct lucet_instance *instance;
    instance = lucet_instance_create(pool, mod, NULL);
    ASSERTm("lucet_instance_create returned NULL", instance != NULL);

    enum lucet_run_stat const stat = lucet_instance_run(instance, "wasm_fault", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state = lucet_instance_get_state(instance);

    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);

    PASS();
}

SUITE(strcmp_suite)
{
    // Tests show that strcmp behaves roughly as expected
    greatest_set_test_suffix("abc_abc");
    RUN_TESTp(strcmp_compare, "abc", "abc");

    greatest_set_test_suffix("def_abc");
    RUN_TESTp(strcmp_compare, "def", "abc");

    greatest_set_test_suffix("abcd_abc");
    RUN_TESTp(strcmp_compare, "abcd", "abc");

    greatest_set_test_suffix("abc_abcd");
    RUN_TESTp(strcmp_compare, "abc", "abcd");

    RUN_TEST(wasm_fault_test);
}
