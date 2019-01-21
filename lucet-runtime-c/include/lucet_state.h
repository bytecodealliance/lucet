#ifndef LUCET_STATE_H
#define LUCET_STATE_H

#include <dlfcn.h>
#include <immintrin.h>
#include <signal.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <ucontext.h>

#include "lucet_export.h"
#include "lucet_instance.h"
#include "lucet_module.h"
#include "lucet_trap.h"
#include "lucet_val.h"

/**
 * Describes the current state of an instance.
 */
enum lucet_state_tag {
    /**
     * Instance is successfully initialized, or has finished running.
     */
    lucet_state_ready = 0,
    /**
     * Instance is currently running.
     * This is the expected state during hostcalls.
     */
    lucet_state_running,
    /**
     * Instance has been terminated by the runtime, because it
     * encountered a fault
     */
    lucet_state_fault,
    /**
     * Instance has been terminated by embedder code, from inside
     * a host call.
     */
    lucet_state_terminated,
};

/**
 * When the guest program returns, the instance is ready to execute again, and
 * the return value is captured here.
 */
struct lucet_state_ready {
    struct lucet_untyped_retval untyped_retval;
};

/**
 * The runtime terminates a guest program when a fault is trapped. This struct
 * describes the details of the fault.
 */
struct lucet_state_fault {
    bool                             fatal;
    struct lucet_trapcode            trapcode;
    uintptr_t                        rip_addr;
    struct lucet_module_addr_details rip_addr_details;
    siginfo_t                        signal_info;
    ucontext_t                       context;
};

/**
 * A host call can terminate a guest program. It can provide a pointer which is
 * up to the embedding to interpret.
 */
struct lucet_state_terminated {
    void *info;
};

/**
 * The state of a WASM guest program.
 *
 * Used to both describe the state of the guest program to API consumers (for
 * example in the case of a faulting guest) and control the state
 * machine in liblucet.
 */
struct lucet_state {
    enum lucet_state_tag tag;
    union {
        struct lucet_state_ready      ready;
        struct lucet_state_fault      fault;
        struct lucet_state_terminated terminated;
    } u;
};

/**
 * Print a string describing the state in char* provided.
 */
int lucet_state_display(char *str, size_t len, struct lucet_state const *) EXPORTED;

/**
 * True unless the sandbox exited or encountered some runtime error?
 */
bool lucet_state_runnable(struct lucet_state const *);

/**
 * True if the instance is in a runtime error state
 */
bool lucet_state_error(struct lucet_state const *);

/**
 * Has an error occured that requires the entire host process to die?
 */
bool lucet_state_fatal(struct lucet_state const *);

/**
 * String representation of an enum lucet_state_tag. Useful for Greatest
 */
const char *lucet_state_name(int tag) EXPORTED;

#endif // LUCET_STATE_H
