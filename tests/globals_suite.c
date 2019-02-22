#include <assert.h>

#include "greatest.h"
#include "lucet.h"
#include "test_helpers.h"

#define DEFINITION_SANDBOX_PATH "globals_guests/definition.so"
#define IMPORT_SANDBOX_PATH "globals_guests/import.so"

TEST defined_globals(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(DEFINITION_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    // Now the globals should be:
    // $x = 3
    // $y = 2
    // $z = 6
    // and heap should be:
    // [0] = 4
    // [4] = 5
    // [8] = 6

    uint8_t *heap = lucet_instance_heap(inst);

    uint32_t global_x_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(4, global_x_read);
    uint32_t global_y_read = ((uint32_t *) heap)[1];
    ASSERT_EQ(5, global_y_read);
    uint32_t global_z_read = ((uint32_t *) heap)[2];
    ASSERT_EQ(6, global_z_read);

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    // now heap should be:
    // [0] = 3
    // [4] = 2
    // [8] = 6

    uint32_t global_x_read_2 = ((uint32_t *) heap)[0];
    ASSERT_EQ(3, global_x_read_2);
    uint32_t global_y_read_2 = ((uint32_t *) heap)[1];
    ASSERT_EQ(2, global_y_read_2);
    uint32_t global_z_read_2 = ((uint32_t *) heap)[2];
    ASSERT_EQ(6, global_z_read_2);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST import_global(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(IMPORT_SANDBOX_PATH), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    // A module that declares import globals will fail instantiate at this time.
    struct lucet_instance *inst;
    const enum lucet_error err = lucet_mmap_region_new_instance(region, mod, &inst);
    ASSERT_ENUM_EQ(lucet_error_unsupported, err, lucet_error_name);

    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
};

SUITE(globals_suite)
{
    RUN_TEST(defined_globals);
    RUN_TEST(import_global);
}
