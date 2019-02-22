#include <assert.h>

#include "greatest.h"
#include "lucet.h"
#include "test_helpers.h"

#define GLOBAL_INIT_SANDBOX_PATH "start_guests/global_init.so"
#define START_AND_CALL_SANDBOX_PATH "start_guests/start_and_call.so"
#define NO_START_SANDBOX_PATH "start_guests/no_start.so"

TEST global_init(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(GLOBAL_INIT_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    // Now the globals should be:
    // $flossie = 17
    // and heap should be:
    // [0] = 17

    uint8_t *heap = lucet_instance_heap(inst);

    uint32_t global_flossie_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(17, global_flossie_read);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST start_and_call(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(START_AND_CALL_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    // Now the globals should be:
    // $flossie = 17
    // and heap should be:
    // [0] = 17

    uint8_t *heap = lucet_instance_heap(inst);

    uint32_t global_flossie_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(17, global_flossie_read);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST no_start(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(NO_START_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    // Now the globals should be:
    // $flossie = 17
    // and heap should be:
    // [0] = 17

    uint8_t *heap = lucet_instance_heap(inst);

    uint32_t global_flossie_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(17, global_flossie_read);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

SUITE(start_suite)
{
    RUN_TEST(global_init);
    RUN_TEST(start_and_call);
    RUN_TEST(no_start);
}
