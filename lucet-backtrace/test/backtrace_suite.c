#include <assert.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include "inttypes.h"

#include "greatest.h"

#include "lucet.h"
#include "lucet_backtrace.h"

#define TEST_SANDBOX_PATH "build/guests/test.so"

void hostcall_fault(void *vmctx)
{
    (void) vmctx;
    *((char *) -1) = 0;
}

TEST test_call_a(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(TEST_SANDBOX_PATH);
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    lucet_instance_run(inst, "a", 1, LUCET_VAL_U64(0));

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);

    struct lucet_backtrace backtrace;
    lucet_backtrace_create(&backtrace, &state->u.fault.context);

    ASSERT(backtrace.count == 3);

    ASSERT_STR_EQ(TEST_SANDBOX_PATH, backtrace.frames[0].file_name);
    ASSERT_STR_EQ("guest_func_b", backtrace.frames[0].sym_name);

    ASSERT_STR_EQ(TEST_SANDBOX_PATH, backtrace.frames[1].file_name);
    ASSERT_STR_EQ("guest_func_a", backtrace.frames[1].sym_name);

    char *find_executable = strstr(backtrace.frames[2].file_name, "build/lucet_test");
    ASSERT(find_executable);
    ASSERT_STR_EQ("lucet_context_backstop", backtrace.frames[2].sym_name);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_call_b(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(TEST_SANDBOX_PATH);
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    lucet_instance_run(inst, "b", 1, LUCET_VAL_U64(1));

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);

    struct lucet_backtrace backtrace;
    lucet_backtrace_create(&backtrace, &state->u.fault.context);

    ASSERT(backtrace.count == 2);

    ASSERT_STR_EQ(TEST_SANDBOX_PATH, backtrace.frames[0].file_name);
    ASSERT_STR_EQ("guest_func_b", backtrace.frames[0].sym_name);

    char *find_executable = strstr(backtrace.frames[1].file_name, "build/lucet_test");
    ASSERT(find_executable);
    ASSERT_STR_EQ("lucet_context_backstop", backtrace.frames[1].sym_name);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_fatal_handler(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(TEST_SANDBOX_PATH);
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    lucet_instance_set_fatal_handler(inst, lucet_backtrace_fatal_handler);

    // Make a fifo to redirect fatal handler output to parent
    char  tmppattern[] = "/tmp/backtrace_suiteXXXXXX";
    char *tmpdir       = mkdtemp(tmppattern);
    ASSERT(tmpdir);
    char *tmpfile = strcat(tmpdir, "/fifo");
    ASSERT(tmpfile);
    int res = mkfifo(tmpfile, S_IRUSR | S_IWUSR);
    ASSERT_EQ(0, res);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Redirect output from the fatal handler into the fifo
        freopen(tmpfile, "w", stderr);
        lucet_instance_run(inst, "call_hostcall_fault", 0);

        exit(1);
    } else {
        FILE *child_stderr = fopen(tmpfile, "r");
        ASSERT(child_stderr);

        int child_status = 0;
        waitpid(child_pid, &child_status, 0);
        // Child will run the fatal handler, which will cause an abort
        ASSERT(WIFSIGNALED(child_status));
        ASSERT_EQ_FMT(SIGABRT, WTERMSIG(child_status), "%d");

        char   buf[1024];
        size_t sz = fread(buf, 1, sizeof(buf), child_stderr);
        ASSERT(sz > 0);

        int  address[6];
        char fault_symbol[128];
        int  items =
            sscanf(buf,
                   "> instance 0x%x had fatal error fault FATAL unknown triggered by Segmentation "
                   "fault: code at address 0x%x (symbol %s (not inside module code) accessed "
                   "memory at 0x%x (inside heap guard)\n"
                   "begin guest backtrace (3 frames)\n"
                   "  ip=%x fname=build/lucet_test sname=hostcall_fault\n"
                   "  ip=%x fname=build/guests/test.so sname=guest_func_call_hostcall_fault\n"
                   "  ip=%x fname=build/lucet_test sname=lucet_context_backstop\n"
                   "end backtrace",
                   &address[0], &address[1], fault_symbol, &address[2], &address[3], &address[4],
                   &address[5]);

        if (items < 7) {
            // sscanf didn't parse the right number of items, maybe the print
            // format has changed?
            fprintf(stderr, "error, unable to parse child stderr:\n%s", buf);
        }
        ASSERT_EQ(7, items);

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

SUITE(backtrace_suite)
{
    RUN_TEST(test_call_a);
    RUN_TEST(test_call_b);
    RUN_TEST(test_fatal_handler);
}
