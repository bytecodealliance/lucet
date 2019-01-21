#include <assert.h>

#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"

#define DEFINITION_SANDBOX_PATH "globals_guests/definition.so"
#define IMPORT_SANDBOX_PATH "globals_guests/import.so"

TEST defined_globals(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(DEFINITION_SANDBOX_PATH));
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

    // Now the globals should be:
    // $x = 3
    // $y = 2
    // $z = 6
    // and heap should be:
    // [0] = 4
    // [4] = 5
    // [8] = 6

    char *heap = lucet_instance_get_heap(inst);

    uint32_t global_x_read = ((uint32_t *) heap)[0];
    ASSERT_EQ(4, global_x_read);
    uint32_t global_y_read = ((uint32_t *) heap)[1];
    ASSERT_EQ(5, global_y_read);
    uint32_t global_z_read = ((uint32_t *) heap)[2];
    ASSERT_EQ(6, global_z_read);

    enum lucet_run_stat const stat2 = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat2, lucet_run_stat_name);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

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
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST import_global(void)
{
    // A module that declares import globals will fail to load
    // at this time.
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(IMPORT_SANDBOX_PATH));
    ASSERT(mod == NULL);

    PASS();
};

SUITE(globals_suite)
{
    RUN_TEST(defined_globals);
    RUN_TEST(import_global);
}
