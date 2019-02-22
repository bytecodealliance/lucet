#include "greatest.h"
#include "lucet.h"
#include "test_helpers.h"

#define LOCALS64_SANDBOX_PATH "stack_guests/locals_64.so"
#define LOCALS_1PAGE_SANDBOX_PATH "stack_guests/locals_1page.so"
#define LOCALS_MULTIPAGE_SANDBOX_PATH "stack_guests/locals_multipage.so"

#define STACK_PER_RECURSION 0xe8

TEST expect_ok(const char *path, int recursion_depth)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "localpalooza", 1,
                                 (struct lucet_val[]){ LUCET_VAL_U32(recursion_depth) }));

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST expect_stack_overflow(const char *path, int recursion_depth)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    const enum lucet_error err = lucet_instance_run(
        inst, "localpalooza", 1, (struct lucet_val[]){ LUCET_VAL_U32(recursion_depth) });

    ASSERT_ENUM_EQ(lucet_error_runtime_fault, err, lucet_error_name);

    struct lucet_state state;
    ASSERT_OK(lucet_instance_state(inst, &state));

    // We should get a nonfatal trap due to the stack overflow.
    ASSERT_ENUM_EQ(lucet_state_tag_fault, state.tag, lucet_state_tag_name);
    ASSERT_EQ(false, state.val.fault.fatal);
    ASSERT_EQ(lucet_trapcode_type_stack_overflow, state.val.fault.trapcode.code);
    ASSERT_EQ(0, state.val.fault.trapcode.tag);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST expect_stack_overflow_probestack(const char *path, int recursion_depth)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    const enum lucet_error err = lucet_instance_run(
        inst, "localpalooza", 1, (struct lucet_val[]){ LUCET_VAL_U32(recursion_depth) });

    ASSERT_ENUM_EQ(lucet_error_runtime_fault, err, lucet_error_name);

    struct lucet_state state;
    ASSERT_OK(lucet_instance_state(inst, &state));

    // We should get a nonfatal trap due to the stack overflow.
    ASSERT_ENUM_EQ(lucet_state_tag_fault, state.tag, lucet_state_tag_name);
    ASSERT_EQ(false, state.val.fault.fatal);
    ASSERT_EQ(lucet_trapcode_type_stack_overflow, state.val.fault.trapcode.code);
    // When the runtime catches probestack, it puts this special tag in the trapcode.
    ASSERT_EQ(UINT16_MAX, state.val.fault.trapcode.tag);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

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
