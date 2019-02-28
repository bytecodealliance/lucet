#include <assert.h>
#include <stdio.h>

#include "greatest.h"
#include "lucet.h"
#include "test_helpers.h"

#define FAULT_MOD_PATH "strcmp_guests/fault_guest.so"

void hostcall_host_fault(void *vmctx)
{
    (void) vmctx;
    char *oob = (char *) -1;
    *oob      = 'x';
}

TEST run_strcmp(const char *mod_path, const char *s1, const char *s2, int *res)
{
    // Test helper function. Runs strcmp in the guest.
    ASSERTm("precondition mod_path", mod_path);
    ASSERTm("precondition s1", s1);
    ASSERTm("precondition s2", s2);
    ASSERTm("precondition res", res);

    const size_t res_size = sizeof(int64_t);
    const size_t s1_size  = strlen(s1) + 1;
    const size_t s2_size  = strlen(s2) + 1;
    ASSERTm("precondition sizes", (res_size + s1_size + s2_size) < LUCET_WASM_PAGE_SIZE);

    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(mod_path), &mod));

    // Pool size reduced from 1000 to 10 because we sometimes can't to get
    // enough virtual memory when running inside docker or on CI.
    struct lucet_test_region *region;
    ASSERT_OK(lucet_test_region_create(10, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_test_region_new_instance(region, mod, &inst));

    uint8_t *heap = lucet_instance_heap(inst);
    uint32_t newpage_start;
    ASSERT_OK(lucet_instance_grow_heap(inst, 1, &newpage_start));

    const guest_ptr_t res_ptr = newpage_start * LUCET_WASM_PAGE_SIZE;
    const guest_ptr_t s1_ptr  = res_ptr + res_size;
    const guest_ptr_t s2_ptr  = s1_ptr + s1_size;
    memcpy(&heap[s1_ptr], s1, s1_size);
    memcpy(&heap[s2_ptr], s2, s2_size);

    ASSERT_OK(lucet_instance_run(inst, "run_strcmp", 3,
                                 (struct lucet_val[]){ LUCET_VAL_GUEST_PTR(s1_ptr),
                                                       LUCET_VAL_GUEST_PTR(s2_ptr),
                                                       LUCET_VAL_GUEST_PTR(res_ptr) }));

    // Copy out state as of program termination
    struct lucet_state state;
    ASSERT_OK(lucet_instance_state(inst, &state));

    if (state.tag == lucet_state_tag_returned) {
        *res = (int) LUCET_UNTYPED_RETVAL_TO_I64(state.val.returned);
    } else {
        // Make sure the result is obviously wrong
        *res = 666;
    }

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_test_region_release(region);

    PASS();
}

TEST strcmp_compare(const char *input1, const char *input2)
{
    // Test compares native strcmp to strcmp running as a guest.
    int res;

    CHECK_CALL(run_strcmp(FAULT_MOD_PATH, input1, input2, &res));

    ASSERT_EQ_FMT(strcmp(input1, input2), res, "%d");

    PASS();
}

TEST wasm_fault_test(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(FAULT_MOD_PATH), &mod));

    struct lucet_test_region *region;
    ASSERT_OK(lucet_test_region_create(10, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_test_region_new_instance(region, mod, &inst));

    const enum lucet_error err = lucet_instance_run(inst, "wasm_fault", 0, (struct lucet_val[]){});
    ASSERT_ENUM_EQ(lucet_error_runtime_fault, err, lucet_error_name);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_test_region_release(region);

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
