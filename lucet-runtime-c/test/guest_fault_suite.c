#include <assert.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

#include "inttypes.h"

#include "greatest.h"

#include "lucet.h"
#include "lucet_vmctx.h"

#include "guest_module.h"
#define TRAPS_SANDBOX_PATH "guest_faults/traps.so"
#define HOSTCALL_ERROR_SANDBOX_PATH "guest_faults/hostcall_error.so"

TEST test_run_illegal_instr(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat stat = lucet_instance_run(inst, "illegal_instr", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_trapcode_bad_signature, state->u.fault.trapcode.code,
                   lucet_trapcode_type_string);

    char state_str[256];
    lucet_state_display(state_str, 256, state);
    char *display_illegal_instr =
        strstr(state_str, "fault bad signature triggered by Illegal instruction: code at address");
    char *display_symbol = strstr(state_str, ":guest_func_illegal_instr) (inside module code)");
    ASSERT_EQ(display_illegal_instr, state_str);
    ASSERT(display_symbol);

    // After a fault, can reset and run a normal function
    lucet_instance_reset(inst);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    stat = lucet_instance_run(inst, "onetwothree", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    ASSERT_EQ(LUCET_UNTYPED_RETVAL_TO_C_INT(state->u.ready.untyped_retval), 123);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_run_oob(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat stat = lucet_instance_run(inst, "oob", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_trapcode_heap_oob, state->u.fault.trapcode.code,
                   lucet_trapcode_type_string);

    char state_str[256];
    lucet_state_display(state_str, 256, state);
    char *display_illegal_instr = strstr(state_str, "fault heap out-of-bounds triggered");
    char *display_symbol        = strstr(state_str, ":guest_func_oob) (inside module code)");
    ASSERT_EQ(display_illegal_instr, state_str);
    ASSERT(display_symbol);

    // After a fault, can reset and run a normal function
    lucet_instance_reset(inst);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    stat = lucet_instance_run(inst, "onetwothree", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    ASSERT_EQ(LUCET_UNTYPED_RETVAL_TO_C_INT(state->u.ready.untyped_retval), 123);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

const char hostcall_test_error[] = "hostcall_test threw an error!";

void hostcall_test(struct lucet_vmctx *ctx)
{
    lucet_vmctx_terminate(ctx, (void *) hostcall_test_error);
}

TEST test_run_hostcall_error(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(HOSTCALL_ERROR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat stat = lucet_instance_run(inst, "main", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_terminated, state->tag, lucet_state_name);
    ASSERT_EQ(state->u.terminated.info, hostcall_test_error);

    char state_str[256];
    lucet_state_display(state_str, 256, state);
    ASSERT_STR_EQ("terminated", state_str);

    // After a fault, can reset and run a normal function
    lucet_instance_reset(inst);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    stat = lucet_instance_run(inst, "onetwothree", 0);
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    ASSERT_EQ(LUCET_UNTYPED_RETVAL_TO_C_INT(state->u.ready.untyped_retval), 123);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_run_fatal_uses_initdata_or_fatal(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Dont show the error output from this fatal fault
        freopen("/dev/null", "w", stderr);
        // Child code should run code that will make an OOB beyond the guard
        // page. This will cause the entire process to abort before returning
        // from lucet_instance_run;
        lucet_instance_run(inst, "fatal", 0);
        // Show that we never get here:
        exit(-2);
    } else {
        int child_status = 0;
        waitpid(child_pid, &child_status, 0);

        // The child runs the default fatal handler, which will call abort.
        ASSERT(WIFSIGNALED(child_status));
        ASSERT_EQ_FMT(SIGABRT, WTERMSIG(child_status), "%d");

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

static void fatal_handler_exit(struct lucet_instance const *i)
{
    (void) i;
    exit(42);
}

TEST test_run_fatal_handler_uses_initdata_or_fatal(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    lucet_instance_set_fatal_handler(inst, fatal_handler_exit);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Child code should run code that will make an OOB beyond the guard
        // page. This will cause the entire process to abort before returning
        // from lucet_instance_run;
        lucet_instance_run(inst, "fatal", 0);
        // Show that we never get here:
        exit(-2);
    } else {
        int child_status = 0;
        waitpid(child_pid, &child_status, 0);

        // The child runs the specified fatal handler, which will exit(42)
        ASSERT(WIFEXITED(child_status));
        ASSERT_EQ_FMT(42, WEXITSTATUS(child_status), "%d");

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

static enum lucet_signal_behavior signal_handler_none(struct lucet_instance *      i,
                                                      struct lucet_trapcode const *trap, int signal,
                                                      void *siginfo, void *uap)
{
    (void) i;
    (void) trap;
    (void) signal;
    (void) siginfo;
    (void) uap;
    return lucet_signal_behavior_none;
}

TEST test_run_fatal_none_signal_handler_uses_initdata_or_fatal(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // This test should behave exactly as before, because the signal handler
    // should return "none"
    lucet_instance_set_signal_handler(inst, signal_handler_none);
    lucet_instance_set_fatal_handler(inst, fatal_handler_exit);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Child code should run code that will make an OOB beyond the guard
        // page. This will cause the entire process to abort before returning
        // from lucet_instance_run;
        lucet_instance_run(inst, "fatal", 0);
        // Show that we never get here:
        exit(-2);
    } else {
        int child_status = 0;
        waitpid(child_pid, &child_status, 0);

        // The child runs the specified fatal handler, which will exit(42)
        ASSERT(WIFEXITED(child_status));
        ASSERT_EQ_FMT(42, WEXITSTATUS(child_status), "%d");

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

static char *recoverable_ptr = NULL;
static void  recoverable_ptr_setup(void)
{
    assert(recoverable_ptr == NULL);
    recoverable_ptr = mmap(NULL, 4096, PROT_NONE, MAP_ANONYMOUS | MAP_PRIVATE, 0, 0);
    assert(recoverable_ptr);
}
static void recoverable_ptr_make_accessible(void)
{
    int res = mprotect(recoverable_ptr, 4096, PROT_READ | PROT_WRITE);
    assert(res == 0);
}

static void recoverable_ptr_teardown(void)
{
    munmap(recoverable_ptr, 4096);
    recoverable_ptr = NULL;
}

char *guest_recoverable_get_ptr(void)
{
    return recoverable_ptr;
}

static enum lucet_signal_behavior signal_handler_continue(struct lucet_instance *      i,
                                                          struct lucet_trapcode const *trap,
                                                          int signal, void *siginfo, void *uap)
{
    (void) i;
    (void) trap;
    (void) siginfo;
    (void) uap;

    // Triggered by a SIGSEGV writing to protected page
    assert(signal == SIGSEGV);
    // The fault was caused by writing to a protected page at recoverable_ptr.
    // Lets make that not a fault anymore.
    recoverable_ptr_make_accessible();

    // Now the guest code can continue.
    return lucet_signal_behavior_continue;
}

TEST test_run_fatal_continue_signal_handler(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // set `recoverable_ptr` to point to a page that is not read/writable
    recoverable_ptr_setup();

    // Install a signal handler that will override the fatal error and tell the
    // sandbox to continue executing. Obviously this is dangerous, but in this
    // case it should be harmless.
    lucet_instance_set_signal_handler(inst, signal_handler_continue);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Child code will call `guest_recoverable_get_ptr` and write to the
        // pointer it returns. This will initially cause a segfault. The signal
        // handler will recover from the segfault, map the page to read/write,
        // and then return to the child code. The child code will then succeed,
        // and the instance will exit successfully.
        enum lucet_run_stat const stat = lucet_instance_run(inst, "recoverable_fatal", 0);
        if (stat != lucet_run_ok) {
            exit(1);
        }

        const struct lucet_state *state;
        state = lucet_instance_get_state(inst);

        if (lucet_state_ready == state->tag) {
            exit(0);
        } else {
            exit(2);
        }
    } else {
        int child_status = 0;
        waitpid(child_pid, &child_status, 0);

        // The child exited 0 because the run finished and the state was as
        // expected:
        ASSERT(WIFEXITED(child_status));
        ASSERT_EQ_FMT(0, WEXITSTATUS(child_status), "%d");

        recoverable_ptr_teardown();

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

static enum lucet_signal_behavior signal_handler_term(struct lucet_instance *      i,
                                                      struct lucet_trapcode const *trap, int signal,
                                                      void *siginfo, void *uap)
{
    (void) i;
    (void) trap;
    (void) siginfo;
    (void) uap;

    // Triggered by a SIGSEGV writing to protected page
    assert(signal == SIGSEGV);

    // Terminate guest
    return lucet_signal_behavior_terminate;
}

TEST test_run_fatal_terminate_signal_handler(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // set `recoverable_ptr` to point to a page that is not read/writable
    recoverable_ptr_setup();

    // Install a signal handler that will override the fatal error and tell the
    // sandbox to exit, but with a nonfatal error. So, we should see an unknown
    // fault
    lucet_instance_set_signal_handler(inst, signal_handler_term);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Child code will call `guest_recoverable_get_ptr` and write to the
        // pointer it returns. This will initially cause a segfault. The pointer
        // is out of the guard pages so it will be a fatal error.
        // The signal not do anything about the segfault, but will make the
        // signal into a terminate, so the unknown trap will return to the host.
        lucet_instance_run(inst, "recoverable_fatal", 0);

        const struct lucet_state *state;
        state = lucet_instance_get_state(inst);

        if (lucet_state_terminated == state->tag) {
            exit(0);
        } else {
            exit(1);
        }
    } else {
        int child_status = 0;
        waitpid(child_pid, &child_status, 0);

        // The child exited 0 because the run finished and the state was as
        // expected:
        ASSERT(WIFEXITED(child_status));
        ASSERT_EQ_FMT(0, WEXITSTATUS(child_status), "%d");

        recoverable_ptr_teardown();

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

static volatile bool host_sigsegv_triggered = false;
void                 test_host_sigsegv_handler(int signal, siginfo_t *info, void *uap)
{
    (void) info;
    (void) uap;
    assert(signal == SIGSEGV);
    recoverable_ptr_make_accessible();
    host_sigsegv_triggered = true;
}

TEST test_sigsegv_handler_saved_restored(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    struct sigaction host_sa;
    host_sa.sa_sigaction = &test_host_sigsegv_handler;
    host_sa.sa_flags     = SA_RESTART | SA_SIGINFO;
    sigfillset(&host_sa.sa_mask);
    int res = sigaction(SIGSEGV, &host_sa, NULL);
    ASSERT(res != -1);

    lucet_instance_run(inst, "illegal_instr", 0);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_fault, state->tag, lucet_state_name);
    ASSERT_ENUM_EQ(lucet_trapcode_bad_signature, state->u.fault.trapcode.code,
                   lucet_trapcode_type_string);

    char state_str[256];
    lucet_state_display(state_str, 256, state);
    char *display_illegal_instr =
        strstr(state_str, "fault bad signature triggered by Illegal instruction: code at address");
    char *display_symbol = strstr(state_str, ":guest_func_illegal_instr) (inside module code)");
    ASSERT_EQ(display_illegal_instr, state_str);
    ASSERT(display_symbol);

    // Now make sure that the host_sa has been restored:
    recoverable_ptr_setup();
    host_sigsegv_triggered = false;

    // accessing this should trigger the segfault:
    *recoverable_ptr = 0;

    ASSERT(host_sigsegv_triggered);

    // Clean up:
    recoverable_ptr_teardown();

    host_sa.sa_handler   = SIG_DFL;
    host_sa.sa_sigaction = NULL;
    host_sa.sa_flags     = SA_RESTART;
    res                  = sigaction(SIGSEGV, &host_sa, NULL);
    ASSERT(res != -1);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

void timeout_handler(int signal)
{
    assert(signal == SIGALRM);
    exit(3);
}

TEST test_alarm(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(TRAPS_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    lucet_instance_set_fatal_handler(inst, fatal_handler_exit);

    pid_t child_pid = fork();
    if (child_pid == 0) {
        // Child code
        // Setup alarm handler. Pend an alarm in 1 second
        signal(SIGALRM, timeout_handler);
        alarm(1);
        // Run guest code that loops forever

        lucet_instance_run(inst, "infinite_loop", 0);
        // Show that we never get here:
        exit(-2);
    } else {
        int child_status = 0;
        // If the test above fails, this might wait forever, and i'm too lazy to
        // do the timeout gynmastics for a unit test that hopefully never fails.
        waitpid(child_pid, &child_status, 0);

        // The child gets the timeout_handler, which exits 3
        ASSERT(WIFEXITED(child_status));
        ASSERT_EQ_FMT(3, WEXITSTATUS(child_status), "%d");

        lucet_instance_release(inst);
        lucet_module_unload(mod);
        lucet_pool_decref(pool);
    }

    PASS();
}

SUITE(guest_fault_suite)
{
    RUN_TEST(test_run_illegal_instr);
    RUN_TEST(test_run_oob);
    RUN_TEST(test_run_hostcall_error);
    RUN_TEST(test_run_fatal_uses_initdata_or_fatal);
    RUN_TEST(test_run_fatal_handler_uses_initdata_or_fatal);
    RUN_TEST(test_run_fatal_none_signal_handler_uses_initdata_or_fatal);
    RUN_TEST(test_run_fatal_continue_signal_handler);
    RUN_TEST(test_run_fatal_terminate_signal_handler);
    RUN_TEST(test_sigsegv_handler_saved_restored);
    RUN_TEST(test_alarm);
}
