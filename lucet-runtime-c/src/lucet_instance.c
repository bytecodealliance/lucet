#include <assert.h>
#include <err.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "lucet_alloc_private.h"
#include "lucet_constants.h"
#include "lucet_data_segment_private.h"
#include "lucet_globals_private.h"
#include "lucet_instance_private.h"
#include "lucet_module_private.h"
#include "lucet_pool_private.h"
#include "lucet_probestack_private.h"
#include "lucet_stats_private.h"
#include "lucet_trap_private.h"

// The host context needs to be available in a thread local variable
// so that the sigaction handler can abort the guest by swapping to the
// host context.
__thread struct lucet_context host_ctx;

// The instance currently being executed via lucet_instance_run. If a signal is
// raised during execution, we need to use this thread-local to determine which
// instance to look up the trap information in, and store the error information
// in.
__thread struct lucet_instance *current_lucet;
// For mocking, we may want to ignore *asserting* that the current_lucet variable
// is set, from a hostcall context, to match the instance we were passed.
bool check_current_lucet = true;

void lucet_instance_unsafe_ignore_current_lucet(bool v)
{
    check_current_lucet = !v;
}

// Store the host sigaction to restore after running.
struct lucet_sig_context {
    stack_t          sigstack;
    struct sigaction sigbus;
    struct sigaction sigsegv;
    struct sigaction sigill;
    struct sigaction sigfpe;
};

static void lucet_handle_signal(int signal, siginfo_t *, void *uap);
static void lucet_signals_on(struct lucet_instance const *, struct lucet_sig_context *);
static void lucet_signals_off(struct lucet_sig_context const *);

static void lucet_instance_fault_detail(struct lucet_instance *i);

static enum lucet_run_stat lucet_instance_run_post_init(struct lucet_instance *i);

struct lucet_vmctx const *lucet_instance_vmctx(struct lucet_instance const *i)
{
    assert(i);
    return (struct lucet_vmctx const *) i->alloc->heap;
}

struct lucet_state const *lucet_instance_get_state(struct lucet_instance const *i)
{
    return (struct lucet_state const *) &i->state;
}

void lucet_instance_set_fatal_handler(struct lucet_instance *i, lucet_fatal_handler *fatal_handler)
{
    i->fatal_handler = fatal_handler;
}

void lucet_instance_set_signal_handler(struct lucet_instance *i,
                                       lucet_signal_handler * signal_handler)
{
    i->signal_handler = signal_handler;
}

static void lucet_instance_default_fatal_handler(struct lucet_instance const *i)
{
    char displaybuf[1024];
    int  res = lucet_state_display(displaybuf, sizeof(displaybuf), &i->state);
    assert(res > 0);

    fprintf(stderr, "> instance %p had fatal error: %s\n", (void *) i, displaybuf);
    // fatal handlers are expected to never return
    abort();
}

struct lucet_instance *lucet_instance_create(struct lucet_pool *pool, struct lucet_module const *m,
                                             void *d_obj)
{
    assert(pool);
    assert(m);

    struct lucet_alloc *alloc = lucet_pool_acquire(pool);
    if (alloc == NULL) {
        // The pool is out of allocations.
        goto error_0;
    }

    // Setup the memory required to run the instance
    enum lucet_alloc_stat const stat = lucet_alloc_allocate_runtime(alloc, &m->runtime_spec);
    if (stat != lucet_alloc_ok) {
        goto error_1;
    }

    struct lucet_instance *i = lucet_alloc_get_instance(alloc);
    *i                       = (struct lucet_instance){
        .magic         = LUCET_INSTANCE_MAGIC,
        .delegate_obj  = d_obj,
        .module        = m,
        .pool          = pool,
        .alloc         = alloc,
        .fatal_handler = lucet_instance_default_fatal_handler,
        .globals       = alloc->globals,
    };

    lucet_instance_reset(i);
    lucet_stats_update(lucet_stat_instantiate, 1);
    return i;

error_1:
    lucet_pool_release(pool, alloc);
error_0:
    lucet_stats_update(lucet_stat_instantiate_fail, 1);
    return NULL;
}

void lucet_instance_reset(struct lucet_instance *i)
{
    lucet_alloc_reset_runtime(i->alloc, i->module);

    lucet_globals_initialize(i->module->runtime_spec.globals, (int64_t *) i->alloc->globals);
    struct lucet_untyped_retval untyped_retval;
    memset(&untyped_retval, 0, sizeof untyped_retval);
    i->state = (struct lucet_state){ .tag     = lucet_state_ready,
                                     .u.ready = { .untyped_retval = untyped_retval } };
}

static enum lucet_run_stat lucet_instance_run_func(struct lucet_instance *   i,
                                                   lucet_module_export_func *func, int argc,
                                                   va_list argv)
{
    if (func == NULL) {
        return lucet_run_symbol_not_found;
    }
    i->entrypoint = func;
    if (lucet_context_init_v(&i->ctx, lucet_alloc_get_stack_top(i->alloc), &host_ctx, func,
                             (void *) lucet_instance_vmctx(i), argc, argv) != 0) {
        return lucet_run_invalid_arguments;
    }
    return lucet_instance_run_post_init(i);
}

enum lucet_run_stat lucet_instance_run(struct lucet_instance *i, const char *entrypoint, int argc,
                                       ...)
{
    assert(i);
    assert(entrypoint);
    assert(argc >= 0);
    lucet_stats_update(lucet_stat_run, 1);

    va_list argv;
    va_start(argv, argc);

    lucet_module_export_func *func = lucet_module_get_export_func(i->module, entrypoint);

    return lucet_instance_run_func(i, func, argc, argv);
}

enum lucet_run_stat lucet_instance_run_start(struct lucet_instance *i)
{
    assert(i);
    lucet_stats_update(lucet_stat_run_start, 1);

    lucet_module_export_func **func_p = (void *) lucet_module_get_start_func(i->module);
    if (func_p == NULL) {
        return lucet_run_symbol_not_found;
    }

    va_list argv;

    return lucet_instance_run_func(i, *func_p, 0, argv);
}

enum lucet_run_stat lucet_instance_run_func_id(struct lucet_instance *i, uint32_t table_id,
                                               uint32_t func_id, int argc, ...)
{
    assert(i);
    lucet_stats_update(lucet_stat_run, 1);

    va_list argv;
    va_start(argv, argc);

    lucet_module_export_func *func = lucet_module_get_func_from_id(i->module, table_id, func_id);

    return lucet_instance_run_func(i, func, argc, argv);
}

static enum lucet_run_stat lucet_instance_run_post_init(struct lucet_instance *i)
{
    // Sandbox is now running:
    i->state.tag = lucet_state_running;

    // current_lucet is a thread-local that gives us access to the running lucet.
    // There should be no sandbox running on this thread when we enter this
    // function
    assert(current_lucet == NULL);
    current_lucet = i;
    // There are special signal handlers for while the guest is executing,
    // when the host context is running we want to defer to the caller's
    // signal handler setup.
    // We save the host context's signal setup to restore later.
    struct lucet_sig_context host_sigs;
    lucet_signals_on(i, &host_sigs);
    // Save the current context into `host_ctx`, and jump to the guest
    // context. The lucet context is linked to host_ctx, so it will return
    // here after it finishes, successfully or otherwise.
    lucet_context_swap(&host_ctx, &i->ctx);
    // Restore the host's signal context.
    lucet_signals_off(&host_sigs);
    current_lucet = NULL;

    // Sandbox has jumped back to the host process, indicating it has
    // either:
    // * trapped, or called hostcall_error: state tag changed
    // * function body returned: set state back to ready
    if (i->state.tag == lucet_state_running) {
        struct lucet_untyped_retval untyped_retval;
        uint64_t                    retval_gp = lucet_context_get_retval_gp(&i->ctx, 0);
        __m128                      retval_fp = lucet_context_get_retval_fp(&i->ctx);
        memcpy(untyped_retval.gp, &retval_gp, sizeof retval_gp);
        _mm_storeu_ps((float *) (void *) untyped_retval.fp, retval_fp);
        i->state = (struct lucet_state){ .tag     = lucet_state_ready,
                                         .u.ready = { .untyped_retval = untyped_retval } };
    }

    // Sandbox is no longer runnable: handle exit, then cleanup thread local
    // Its unsafe to determine all error details in signal handler, so we fill in
    // information we got from dladdr here:
    if (i->state.tag == lucet_state_fault) {
        lucet_instance_fault_detail(i);
    }

    // Some errors indicate that the guest is not functioning correctly or
    // that the loaded code violated some assumption:
    if (lucet_state_fatal(&i->state)) {
        i->fatal_handler(i);
        abort();
    }

    if (lucet_state_error(&i->state)) {
        lucet_stats_update(lucet_stat_exit_error, 1);
    } else {
        lucet_stats_update(lucet_stat_exit_ok, 1);
    }

    return lucet_run_ok;
}

void lucet_instance_release(struct lucet_instance *i)
{
    assert(i);
    assert(i->pool);

    struct lucet_alloc *alloc = i->alloc;
    lucet_alloc_free_runtime(alloc);

    struct lucet_pool *pool = i->pool;
    lucet_pool_release(pool, alloc);
    lucet_stats_update(lucet_stat_release_instance, 1);
}

char *lucet_instance_get_heap(struct lucet_instance const *i)
{
    return lucet_alloc_get_heap(i->alloc);
}

bool lucet_instance_check_heap(struct lucet_instance const *i, void *ptr, size_t len)
{
    return lucet_alloc_mem_in_heap(i->alloc, ptr, len);
}

void *lucet_instance_get_delegate(struct lucet_instance const *i)
{
    return i->delegate_obj;
}

uint32_t lucet_instance_current_memory(struct lucet_instance const *inst)
{
    assert(inst);
    uint32_t heap_len = lucet_alloc_get_heap_len(inst->alloc);
    return heap_len / LUCET_WASM_PAGE_SIZE;
}

int32_t lucet_instance_grow_memory(struct lucet_instance *inst, uint32_t additional_pages)
{
    assert(inst);
    int64_t orig_len =
        lucet_alloc_expand_heap(inst->alloc, additional_pages * LUCET_WASM_PAGE_SIZE);
    if (orig_len < 0) {
        return -1;
    } else {
        return orig_len / LUCET_WASM_PAGE_SIZE;
    }
}

void lucet_instance_terminate(struct lucet_instance *i, void *info)
{
    i->state = (struct lucet_state){
        .tag = lucet_state_terminated,
        .u.terminated =
            (struct lucet_state_terminated){
                .info = info,
            },
    };
    lucet_context_set(&host_ctx);
}

// Private. Only safe to call when inside guest ctx
struct lucet_instance *lucet_vmctx_instance(struct lucet_vmctx const *vmctx)
{
    if (vmctx == NULL) {
        err(1, "%s() null vmctx", __FUNCTION__);
    }

    uintptr_t inst = ((uintptr_t) vmctx) - lucet_alloc_instance_heap_offset;
    // We shouldn't actually need to access the thread local, only the exception
    // handler should need to. But, as long as the thread local exists, we
    // should make sure that the guest hasn't pulled any shenanigans and passed
    // a bad vmctx. (Codegen should ensure the guest cant pull any shenanigans
    // but there have been bugs before.)
    if (inst != (uintptr_t) current_lucet && check_current_lucet) {
        if (current_lucet == NULL) {
            errx(1,
                 "%s() current_lucet is NULL. thread local storage failure can indicate dynamic "
                 "linking issue",
                 __FUNCTION__);
        } else {
            errx(1, "%s() vmctx did not correspond to current_lucet", __FUNCTION__);
        }
    }
    return (struct lucet_instance *) inst;
}

static void lucet_signals_on(struct lucet_instance const *i, struct lucet_sig_context *save)
{
    int res;

    // Setup signal stack
    stack_t sigstack;
    lucet_alloc_get_sigstack(i->alloc, &sigstack);
    res = sigaltstack(&sigstack, &save->sigstack);
    assert(res != -1);

    // Setup signal handlers
    struct sigaction sa;
    sa.sa_sigaction = &lucet_handle_signal;
    sa.sa_flags     = SA_RESTART | SA_SIGINFO | SA_ONSTACK;
    sigfillset(&sa.sa_mask);
    res = sigaction(SIGBUS, &sa, &save->sigbus);
    assert(res != -1);
    res = sigaction(SIGSEGV, &sa, &save->sigsegv);
    assert(res != -1);
    res = sigaction(SIGILL, &sa, &save->sigill);
    assert(res != -1);
    res = sigaction(SIGFPE, &sa, &save->sigfpe);
    assert(res != -1);
}

static void lucet_signals_off(struct lucet_sig_context const *restore)
{
    int res;
    // Restore signal handlers
    res = sigaction(SIGBUS, &restore->sigbus, NULL);
    assert(res != -1);
    res = sigaction(SIGSEGV, &restore->sigsegv, NULL);
    assert(res != -1);
    res = sigaction(SIGILL, &restore->sigill, NULL);
    assert(res != -1);
    res = sigaction(SIGFPE, &restore->sigfpe, NULL);
    assert(res != -1);
    // Restore signal stack
    res = sigaltstack(&restore->sigstack, NULL);
    assert(res != -1);
}

static void lucet_instance_fault_detail(struct lucet_instance *i)
{
    assert(i->state.tag == lucet_state_fault);
    uintptr_t const rip = i->state.u.fault.rip_addr;

    // We do this after returning from the signal handler because it
    // requires `dladdr` calls, which are not signal safe.
    lucet_module_get_addr_details(i->module, &i->state.u.fault.rip_addr_details, rip);

    struct lucet_state *s = &i->state;

    // If the trap table lookup returned unknown, it is a fatal error.
    if (s->u.fault.trapcode.code == lucet_trapcode_unknown) {
        s->u.fault.fatal = true;
        return;
    }

    // If the trap was a segv or bus fault and the addressed memory was
    // outside the guard pages, it is a fatal error.
    siginfo_t *siginfo = &s->u.fault.signal_info;
    if ((siginfo->si_signo == SIGSEGV || siginfo->si_signo == SIGBUS) &&
        !lucet_alloc_addr_in_heap_guard(i->alloc, (uintptr_t) siginfo->si_addr)) {
        s->u.fault.fatal = true;
        return;
    }
}

static void lucet_handle_signal(int signal, siginfo_t *info, void *uap)
{
    /*
     * This function is only designed to handle signals that are the direct
     * result of execution of a hardware instruction from the faulting WASM
     * thread. It thus safely assumes the signal is directed specifically
     * at this thread (i.e. not a different thread or the process as a whole).
     */
    assert(signal == SIGBUS || signal == SIGSEGV || signal == SIGILL || signal == SIGFPE);
    assert(info);

    struct lucet_instance *i = current_lucet;

    ucontext_t *    ctx = (ucontext_t *) uap;
    uintptr_t const rip = (uintptr_t) ctx->uc_mcontext.gregs[REG_RIP];

    struct lucet_trapcode trapcode;
    if (i->module->trap_manifest.records == NULL) {
        // There is no trap manifest, so we cannot look up a trapcode.
        // This will be converted to a fatal trap after we switch back
        // to the host.
        trapcode = (struct lucet_trapcode){
            .code = lucet_trapcode_unknown,
            .tag  = 0,
        };
    } else {
        trapcode = lucet_trap_lookup(&i->module->trap_manifest, rip);
    }

    // Default behavior is `none` - handler may return that
    enum lucet_signal_behavior behavior = lucet_signal_behavior_none;
    if (i->signal_handler) {
        behavior = i->signal_handler(i, &trapcode, signal, info, uap);
        // Assert that result is a valid member of the enum
        assert(behavior <= lucet_signal_behavior_terminate);
    }

    if (behavior == lucet_signal_behavior_continue) {
        // Return to the guest context without making any modifications
        // to instance.
        return;
    } else if (behavior == lucet_signal_behavior_terminate) {
        // Set the t
        i->state = (struct lucet_state){ .tag          = lucet_state_terminated,
                                         .u.terminated = {
                                             .info = NULL,
                                         } };
    } else {
        // otherwise, record the fault and jump back to the host context
        i->state = (struct lucet_state){
            .tag = lucet_state_fault,
            .u.fault =
                {
                    // fatal field is set false here by default - have to wait until
                    // `verify_trap_safety` to have enough information to determine
                    // whether trap was fatal. It is not signal safe to access some of
                    // the required information.
                    .fatal       = false,
                    .trapcode    = trapcode,
                    .rip_addr    = rip,
                    .signal_info = *info,
                    .context     = *ctx,
                },
        };
    }
    // Jump back to the host context
    lucet_context_set_from_signal(&host_ctx);
}

// print the enum as a string
const char *lucet_run_stat_name(int stat)
{
    switch (stat) {
    case lucet_run_ok:
        return "ok";
    case lucet_run_symbol_not_found:
        return "symbol not found";
    default:
        return "<invalid>";
    }
}

// print the enum as a string
const char *lucet_signal_behavior_name(int sig_beh)
{
    switch (sig_beh) {
    case lucet_signal_behavior_none:
        return "none";
    case lucet_signal_behavior_continue:
        return "continue";
    case lucet_signal_behavior_terminate:
        return "terminate";
    default:
        return "<invalid>";
    }
}
