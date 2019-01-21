#include <assert.h>
#include <err.h>

#include "../include/lucet.h"
#include "greatest.h"
#include "guest_module.h"

#define SUCCESS_SANDBOX_PATH "stats/success.so"
#define NONEXISTENT_SANDBOX_PATH "stats/does_not_exist.so"
struct my_stats {
    int64_t load;
    int64_t load_fail;
    int64_t unload;
    int64_t instantiate;
    int64_t instantiate_fail;
    int64_t run_start;
    int64_t run;
    int64_t exit_ok;
    int64_t exit_error;
    int64_t release;
};

struct my_stats my_stats;
void            my_stats_callback(enum lucet_stat_type stat_type, int64_t val); // forward decl

TEST test_stats_update(void)
{
    // Set up our custom stats callback
    lucet_stats_set_callback(my_stats_callback);

    struct lucet_module *mod_success, *mod_ne;

    // +1 load_fail
    mod_ne = lucet_module_load(guest_module_path(NONEXISTENT_SANDBOX_PATH));
    ASSERT(mod_ne == NULL);
    // +1 load
    mod_success = lucet_module_load(guest_module_path(SUCCESS_SANDBOX_PATH));
    ASSERT(mod_success != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst, *inst2;
    // +1 instantiate
    inst = lucet_instance_create(pool, mod_success, NULL);
    ASSERT(inst != NULL);

    // Intentially fail an instantiate (pool is empty)
    // +1 instantiate_fail
    inst2 = lucet_instance_create(pool, mod_success, NULL);
    ASSERT(inst2 == NULL);

    // +1 run_start
    lucet_instance_run_start(inst);

    // +1 run, exit_ok
    lucet_instance_run(inst, "main", 0);

    // +1 release_instance
    lucet_instance_release(inst);

    // +1 unload
    lucet_module_unload(mod_success);
    lucet_pool_decref(pool);

    // Make sure the stats that were output match our expectations.
    ASSERT_EQ(1, my_stats.load);
    ASSERT_EQ(1, my_stats.load_fail);
    ASSERT_EQ(1, my_stats.unload);
    ASSERT_EQ(1, my_stats.instantiate);
    ASSERT_EQ(1, my_stats.instantiate_fail);
    ASSERT_EQ(1, my_stats.run_start);
    ASSERT_EQ(1, my_stats.run);
    ASSERT_EQ(1, my_stats.exit_ok);
    ASSERT_EQ(0, my_stats.exit_error);
    ASSERT_EQ(1, my_stats.release);

    PASS();
}

void my_stats_callback(enum lucet_stat_type stat_type, int64_t val)
{
    switch (stat_type) {
    case lucet_stat_program_load:
        my_stats.load += val;
        break;
    case lucet_stat_program_load_fail:
        my_stats.load_fail += val;
        break;
    case lucet_stat_program_unload:
        my_stats.unload += val;
        break;
    case lucet_stat_instantiate:
        my_stats.instantiate += val;
        break;
    case lucet_stat_instantiate_fail:
        my_stats.instantiate_fail += val;
        break;
    case lucet_stat_run:
        my_stats.run += val;
        break;
    case lucet_stat_run_start:
        my_stats.run_start += val;
        break;
    case lucet_stat_exit_ok:
        my_stats.exit_ok += val;
        break;
    case lucet_stat_exit_error:
        my_stats.exit_error += val;
        break;
    case lucet_stat_release_instance:
        my_stats.release += val;
        break;
    default:
        errx(1, "%s() unexpected stat type", __FUNCTION__);
    }
}

SUITE(stats_suite)
{
    RUN_TEST(test_stats_update);
}
