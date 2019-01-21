/**
 * @file lucet_instance.h
 * @brief Functions and data structures relating to the use of liblucet "lucet_instance" structures.
 */

#ifndef LUCET_INSTANCE_H
#define LUCET_INSTANCE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lucet_decls.h"
#include "lucet_export.h"
#include "lucet_trap.h"
#include "lucet_val.h"

/**
 * Create and initialize an lucet_instance.
 * Gets the memory for the instance from the lucet_pool. Returns NULL if the pool
 * is empty.
 *
 * Instantiates the wasm guest program described in the lucet_module.
 * Takes a delegate pointer that will be associated with the instance. This
 * pointer can be retrieved from inside a hostcall with lucet_vmctx_get_delegate,
 * or the lucet_instance_get_delegate below. This pointer is only used by the
 * embedding.
 */
struct lucet_instance *lucet_instance_create(struct lucet_pool *        pool,
                                             struct lucet_module const *module,
                                             void *                     delegate_obj) EXPORTED;

/**
 * Release an lucet_instance back into the pool it came from.
 */
void lucet_instance_release(struct lucet_instance *) EXPORTED;

/**
 * Return code for lucet_instance_run indicates whether the specified symbol was
 * found in the module. This doesn't tell you about the status of the execution,
 * use lucet_state for that.
 */
enum lucet_run_stat {
    /**
     * Symbol was found
     */
    lucet_run_ok,
    /**
     * Requested symbol was not found in the module. Execution did not start.
     */
    lucet_run_symbol_not_found,
    /**
     * Invalid arguments. Execution did not start.
     */
    lucet_run_invalid_arguments,
};

/**
 * Run an instance. Provide the name of the entrypoint, and the arguments that
 * entrypoint takes, as lucet_val values.
 */
enum lucet_run_stat lucet_instance_run(struct lucet_instance *, const char *entrypoint, int argc,
                                       ...) EXPORTED;

/**
 * Run the start section of an instance, if present. Returns
 * `lucet_run_ok` on success, or `lucet_run_symbol_not_found` if there was
 * no `guest_start` symbol in the instance.
 *
 * TODO: we should figure out soon how/when to call this
 * unconditionally, as programs that use it will behave incorrectly
 * without it being run. Potential options:
 *
 * 1. In `lucet_instance_run`, with a runonce flag on the instance
 *     a. Call this function from `run`
 *     b. Set up the stack so the start section runs on the first bootstrap
 *
 * 2. In `lucet_instance_create`
 */
enum lucet_run_stat lucet_instance_run_start(struct lucet_instance *i) EXPORTED;

/**
 * Run a function given its ID
 */
enum lucet_run_stat lucet_instance_run_func_id(struct lucet_instance *i, uint32_t table_id,
                                               uint32_t func_id, int argc, ...) EXPORTED;

/**
 * Reset an instance. Restores all instance state (heap, globals, lucet_state,
 * return values, memory limit) to what they were when the instance was created.
 * Does not clear signal or fatal handlers, or delegate object.
 */
void lucet_instance_reset(struct lucet_instance *) EXPORTED;

/**
 * Get a pointer to the lucet_state for the instance. The lucet_state tells you
 * about the execution of the instance - if its been run yet, whether it exited,
 * or if any runtime errors occured.
 */
struct lucet_state const *lucet_instance_get_state(struct lucet_instance const *i) EXPORTED;

/**
 * Get a pointer to the heap for reading and writing the instance's memory. Make
 * sure any pointers into the heap are valid with `lucet_instance_check_heap` below.
 */
char *lucet_instance_get_heap(struct lucet_instance const *i) EXPORTED;

/**
 * Check that a memory region exists inside the instance's heap.
 */
bool lucet_instance_check_heap(struct lucet_instance const *i, void *ptr, size_t len) EXPORTED;

/**
 * returns the current number of wasm pages
 */
uint32_t lucet_instance_current_memory(struct lucet_instance const *) EXPORTED;

/**
 * takes the number of wasm pages to grow by. returns the number of pages before
 * the call on success, or -1 on failure.
 */
int32_t lucet_instance_grow_memory(struct lucet_instance *, uint32_t additional_pages) EXPORTED;

/**
 * Get the delegate object for a given instance. This was passed in at the
 * create call.
 */
void *lucet_instance_get_delegate(struct lucet_instance const *) EXPORTED;

/**
 * Enumerates the different behaviors that a signal handler can trigger in an
 * instance.
 */
enum lucet_signal_behavior {
    /**
     * Use default behavior.
     */
    lucet_signal_behavior_none = 0,
    /**
     * Override default behavior and cause the instance to continue.
     */
    lucet_signal_behavior_continue,
    /**
     * Override default behavior and cause the instance to terminate.
     */
    lucet_signal_behavior_terminate,
};

/**
 * Type for functions that are to be used to handle signals.
 */
typedef enum lucet_signal_behavior lucet_signal_handler(struct lucet_instance *,
                                                        struct lucet_trapcode const *trap,
                                                        int signal, void *siginfo, void *uap);

/**
 * Type for functions that are to be used to handle fatal traps.
 */
typedef void lucet_fatal_handler(struct lucet_instance const *);

/**
 * Register a fatal handler for the instance.
 *
 * If an *unexpected* error occurs, the instance will exit in a fatal state, as
 * indicated by the `lucet_state_fatal` function. A fatal state indicates that it
 * is NOT safe for the host environment to continue. A fatal state can be
 * induced in a number of different ways: an out-of-bounds access that falls
 * outside the expected heap and guard, a trap with an instruction pointer that
 * does not have a corresponding trap site, or just via the `fatal` flag on
 * hostcall_error.
 *
 * Note that fundamentally we cannot guarantee that this handler will be run on
 * a fatal error. If an error occurs that we would be consider to be fatal we
 * may not even be able to catch it, as it is by its very definition unexpected.
 *
 * The fatal handler function should be kept as minimal as possible and must
 * cause the host environment to exit and thus never return. It is intended to
 * be used for printing backtraces and error info and then exiting. If it does
 * return, the host environment will immediately be aborted.
 */
void lucet_instance_set_fatal_handler(struct lucet_instance *,
                                      lucet_fatal_handler *fatal_handler) EXPORTED;

/**
 * Register a signal handler for the instance.
 *
 * Liblucet registers a signal handler for SIGBUS, SIGSEGV, SIGILL, and SIGFPE
 * while the guest code is actively running (inside `lucet_instance_run`). The
 * library needs to intercept these signals in order to determine the
 * `lucet_state_error_trap`
 *
 * In the signal handler, liblucet first determines the `lucet_trapcode` for the
 * signal.  When no `lucet_signal_handler` is registered, or the `lucet_signal_handler`
 * returns `lucet_signal_behavior_none`, the liblucet behavior is intended to fail
 * safe. If the code `type` is `lucet_trapcode_unknown`, that means that the
 * library could not determine why the signal happened, which means that the
 * sandbox may be breached and will be shut down with a fatal error (see
 * `lucet_instance_set_fatal_handler` above). Additionally, an `lucet_trapcode_oob`
 * can be a fatal error if the access is to memory that is not an appropriate
 * guard page managed by liblucet. If a trap is fatal, liblucet will terminate the
 * host process by calling `abort`. Otherwise, if the trap is nonfatal, it
 * indicates the sandbox trapped a behavior of the guest code that is intended
 * to terminate the guest without harming the host.
 *
 * The non-default signal behaviors exist so that the host embedding can handle
 * signals generated by host code running from inside the guest context (which
 * we usually refer to as a "host call") may be managed.
 *
 * The lucet_signal_handler is passed all signals that occur, and the trap
 * information associated with them. The handler can inspect signals with trap
 * code type `lucet_trapcode_unknown` or `lucet_trapcode_oob`, and use the `siginfo`
 * and `uap` arguments to determine if host code is responsible for the fault.
 * In these cases, the handler code may choose to remedy the cause of the trap
 * and return to the code running in the guest context by returning
 * `lucet_signal_behavior_continue`. Alternately, the host can return
 * `lucet_behavior_nonfatal_exit`, which will terminate the guest context but
 * indicate to liblucet to not determine whether the fault should terminate the
 * entire process, and leave it up to the host embedding to deal with the fault
 * when it returns from `lucet_instance_run`.
 *
 * The `lucet_signal_handler` is passed the instance and the trapcode determined
 * by liblucet, as well as the signal number, siginfo_t, and ucontext_t that the
 * operating system passed to the `sigaction` handler registered by the library.
 * The siginfo_t for `siginfo` and ucontext_t for `uap` are void* in this
 * prototype so that their headers don't have to be included by all users of
 * liblucet.
 */
void lucet_instance_set_signal_handler(struct lucet_instance *,
                                       lucet_signal_handler *signal_handler) EXPORTED;

/**
 * Turn an lucet_run_stat into a string.
 */
const char *lucet_run_stat_name(int lucet_run_stat) EXPORTED;

/**
 * Turn an lucet_signal_behavior into a string
 */
const char *lucet_signal_behavior_name(int lucet_signal_behavior) EXPORTED;

/**
 * For implementing test harnesses only! super dangerous! overrides the check
 * that methods on vmctx are only used when an instance is being executed
 * with lucet_instance_run.
 */
void lucet_instance_unsafe_ignore_current_lucet(bool) EXPORTED;

#endif // LUCET_INSTANCE_H
