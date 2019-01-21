#include <string.h>

#include "inttypes.h"

#include "features/session.h"
#include "greatest.h"
#include "guest_module.h"

#include "../include/lucet.h"
#define HELLO_MOD_PATH "features/hello.so"

TEST run(const char *mod_path, struct session *session, struct lucet_state *exit_state)
{
    struct lucet_pool *pool = lucet_pool_create(1, NULL);
    ASSERTm("failed to create pool", pool != NULL);
    struct lucet_module *mod = lucet_module_load(guest_module_path(mod_path));
    ASSERTm("failed to load module", mod != NULL);

    // Now we have all the ingredients to create an instance and run it
    struct lucet_instance *instance = lucet_instance_create(pool, mod, session);

    ASSERTm("lucet_instance_create returned NULL", instance != NULL);

    lucet_instance_run(instance, "main", 0);

    // Copy out state as of program termination
    const struct lucet_state *state;
    state = lucet_instance_get_state(instance);
    memcpy(exit_state, state, sizeof(struct lucet_state));

    lucet_instance_release(instance);

    lucet_module_unload(mod);

    lucet_pool_decref(pool);
    PASS();
}

TEST run_by_func_id(const char *mod_path, struct session *session, struct lucet_state *exit_state)
{
    struct lucet_pool *pool = lucet_pool_create(1, NULL);
    ASSERTm("failed to create pool", pool != NULL);
    struct lucet_module *mod = lucet_module_load(guest_module_path(mod_path));
    ASSERTm("failed to load module", mod != NULL);

    // Now we have all the ingredients to create an instance and run it
    struct lucet_instance *instance = lucet_instance_create(pool, mod, session);

    ASSERTm("lucet_instance_create returned NULL", instance != NULL);

    lucet_instance_run_func_id(instance, 0, 0, 0, NULL);

    // Copy out state as of program termination
    const struct lucet_state *state;
    state = lucet_instance_get_state(instance);
    memcpy(exit_state, state, sizeof(struct lucet_state));

    lucet_instance_release(instance);

    lucet_module_unload(mod);

    lucet_pool_decref(pool);
    PASS();
}

TEST session_1_uses_initdata_or_fatal(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) "X-Sandbox:1");

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_ready, end_state.tag, lucet_state_name);

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_native.c!\n"
                   "hello again from sandbox_native.c!\n"
                   "got sandbox key: 1\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST session_2_uses_initdata_or_fatal(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) "X-Sandbox:2");

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_ready, end_state.tag, lucet_state_name);

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_native.c!\n"
                   "hello again from sandbox_native.c!\n"
                   "got sandbox key: 2\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST session_3_uses_initdata_or_fatal(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) "X-Sandbox:3");

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_terminated, end_state.tag, lucet_state_name);
    ASSERT_EQ(-1, (int64_t) end_state.u.terminated.info);

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_native.c!\n"
                   "hello again from sandbox_native.c!\n"
                   "got sandbox key: 3\n"
                   "going to exit with code -1\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST session_4_uses_initdata_or_fatal(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) "X-Sandbox:4");

    CHECK_CALL(run_by_func_id(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_ready, end_state.tag, lucet_state_name);

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_native.c!\n"
                   "hello again from sandbox_native.c!\n"
                   "got sandbox key: 4\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

SUITE(features_suite)
{
    RUN_TEST(session_1_uses_initdata_or_fatal);
    RUN_TEST(session_2_uses_initdata_or_fatal);
    RUN_TEST(session_3_uses_initdata_or_fatal);
    RUN_TEST(session_4_uses_initdata_or_fatal);
}
