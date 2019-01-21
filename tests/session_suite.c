#include <assert.h>
#include <stdio.h>

#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"
#include "lucet_libc.h"
#include "session.h"

#define HELLO_MOD_PATH "session_guests/hello.so"
#define ALLOC_MOD_PATH "session_guests/alloc.so"
#define STDIO_MOD_PATH "session_guests/stdio.so"

static const char *request_header[] = {
    "X-Sandbox:1",
    "X-Sandbox:2",
    "X-Sandbox:3",
    "X-Sandbox:4",
};

static void session_stdio_handler(struct lucet_libc *libc, int32_t fd, const char *buf, size_t len)
{
    struct session *sess = (struct session *) libc;
    session_stdio_write(sess, fd, buf, len);
}

TEST run(const char *mod_path, struct session *session, struct lucet_state *exit_state)
{
    struct lucet_pool *pool = lucet_pool_create(1, NULL);
    ASSERTm("failed to create pool", pool != NULL);
    struct lucet_module *mod = lucet_module_load(guest_module_path(mod_path));
    ASSERTm("failed to load module", mod != NULL);

    // Now we have all the ingredients to create an instance, and run it
    struct lucet_instance *instance;
    instance = lucet_instance_create(pool, mod, session);

    lucet_libc_set_stdio_handler(&session->libc, session_stdio_handler);

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

TEST test_run_session_hello_0(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) request_header[0]);

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_terminated, end_state.tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_libc_term_exit, session.libc.term_reason, lucet_libc_term_reason_str);
    ASSERT_EQ(0, lucet_libc_exit_code(&session.libc));

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_hello.c!\n"
                   "got sandbox key: 1\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST test_run_session_hello_1(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) request_header[1]);

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_terminated, end_state.tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_libc_term_exit, session.libc.term_reason, lucet_libc_term_reason_str);
    ASSERT_EQ(0, lucet_libc_exit_code(&session.libc));

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_hello.c!\n"
                   "got sandbox key: 2\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST test_run_session_hello_2(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) request_header[2]);

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_terminated, end_state.tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_libc_term_exit, session.libc.term_reason, lucet_libc_term_reason_str);
    ASSERT_EQ(-1, lucet_libc_exit_code(&session.libc));

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_hello.c!\n"
                   "got sandbox key: 3\n"
                   "sandbox is going to exit with -1\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST test_run_session_hello_3(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) request_header[3]);

    CHECK_CALL(run(HELLO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_fault, end_state.tag, lucet_state_name);

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_hello.c!\n"
                   "got sandbox key: 4\n"
                   "sandbox is going to access invalid memory\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST test_run_session_alloc(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) request_header[0]);

    CHECK_CALL(run(ALLOC_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_ready, end_state.tag, lucet_state_name);

    ASSERT_STR_EQm("session output",
                   "hello from sandbox_alloc.c!\n"
                   "got sandbox key: 1\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

TEST test_run_session_stdio(void)
{
    struct lucet_state end_state;
    struct session     session = { 0 };
    session_create(&session, (const unsigned char *) request_header[0]);

    CHECK_CALL(run(STDIO_MOD_PATH, &session, &end_state));

    ASSERT_ENUM_EQ(lucet_state_terminated, end_state.tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_libc_term_exit, session.libc.term_reason, lucet_libc_term_reason_str);
    ASSERT_EQ(0, lucet_libc_exit_code(&session.libc));

    ASSERT_STR_EQm("session output",
                   "stdio 1 > hello, stdout!\n"
                   "stdio 2 > hello, stderr!\n"
                   "stdio 1 > snprintf can format digits: 12345 and strings: \"teststr\"\n",
                   session.output);

    session_destroy(&session);
    PASS();
}

SUITE(session_suite)
{
    RUN_TEST(test_run_session_hello_0);
    RUN_TEST(test_run_session_hello_1);
    RUN_TEST(test_run_session_hello_2);
    RUN_TEST(test_run_session_hello_3);

    RUN_TEST(test_run_session_alloc);

    RUN_TEST(test_run_session_stdio);
}
