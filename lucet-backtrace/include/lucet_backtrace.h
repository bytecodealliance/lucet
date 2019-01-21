#ifndef LUCET_BACKTRACE_H
#define LUCET_BACKTRACE_H

#include "lucet_decls.h"
#include "lucet_export.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <ucontext.h>

/**
 * The max number of frames that are captured in the backtrace from a faulting
 * guest
 */
#define LUCET_MAX_BACKTRACE_DEPTH 32

/**
 * The number of registers stored for each stack frame in the backtrace
 * Note: Registers are indexed by libunwind see its src for details
 */
#define LUCET_BACKTRACE_N_REGS 17

/**
 * Metadata about processor registers stored in a backtrace.
 */
struct lucet_backtrace_reg {
    uint64_t    value;
    const char *name;  // statically allocated; see unw_regname(3)
    bool        valid; // set if the register value could be recovered
};

/**
 * Metadata about a stack frame stored in a backtrace.
 * Some information is populated in the fault handler, while other data
 * is fleshed out in the "details" functions (since it may not be safe to
 * gather it in a signal handler context).
 */
struct lucet_backtrace_frame {
    uint64_t                   ip;
    struct lucet_backtrace_reg regs[LUCET_BACKTRACE_N_REGS];
    // Best effort is made to get file and symbol name.
    const char *file_name;
    const char *sym_name;
};

/**
 * A backtrace from a faulting guest program.
 */
struct lucet_backtrace {
    uint64_t                     count;
    struct lucet_backtrace_frame frames[LUCET_MAX_BACKTRACE_DEPTH];
};

/**
 * Stores execution backtrace to structure.
 */
void lucet_backtrace_create(struct lucet_backtrace *backtrace, const ucontext_t *ctx) EXPORTED;

/**
 * Print a backtrace to a file handle. Verbose boolean determines if registers
 * are printed as well.
 */
void lucet_backtrace_print(struct lucet_backtrace const *backtrace, bool verbose,
                           FILE *file) EXPORTED;

/**
 * A fatal handler (for use with `lucet_instance_set_fatal_handler`) that prints a
 * backtace (using `lucet_backtrace_print`) to stderr.
 */
void lucet_backtrace_fatal_handler(struct lucet_instance const *) EXPORTED;

#endif // LUCET_BACKTRACE_H
