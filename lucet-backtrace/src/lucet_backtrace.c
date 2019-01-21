
#include <assert.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

#include "lucet_backtrace.h"

#define UNW_LOCAL_ONLY
#include <libunwind.h>

#include "lucet_instance.h"
#include "lucet_state.h"

void lucet_backtrace_create(struct lucet_backtrace *backtrace, const ucontext_t *ctx)
{
    assert(backtrace);

    /* From unw_getcontext(3) man page:
     * On IA-64, unw_context_t has a layout that is compatible with that of
     * ucontext_t and such structures can be initialized with getcontext()
     * instead of unw_getcontext(). However, the reverse is not true ... */
    unw_context_t *uc = (unw_context_t *) ctx;

    unw_cursor_t cursor;
    int          res = unw_init_local(&cursor, uc);
    if (res != 0) {
        goto error;
    }

    int i = 0;
    do {
        // Read instruction pointer register explicitly. The IP is needed for
        // the frame to be at all meaningful. and it is assumed to be set
        uint64_t ip;
        int      err = unw_get_reg(&cursor, UNW_REG_IP, &ip);
        if (err) {
            fprintf(stderr,
                    "WARNING: backtrace frame %i omitted due to error "
                    "reading instruction pointer: unw_get_reg returned %d\n",
                    i, err);
            continue;
        }
        backtrace->count        = i;
        backtrace->frames[i].ip = ip;

        // Get symbol info for the instruction pointer
        Dl_info dli;
        int     res = dladdr((void *) ip, &dli);
        if (res > 0) {
            backtrace->frames[i].file_name = dli.dli_fname;
            backtrace->frames[i].sym_name  = dli.dli_sname;
        }

        // Store register values (as available). Note that registers are
        // indexed by libunwind and there may be more than LUCET_BACKTRACE_N_REGS
        // available; see libunwind src for details
        for (int j = 0; j < LUCET_BACKTRACE_N_REGS; j++) {
            err = unw_get_reg(&cursor, j, &backtrace->frames[i].regs[j].value);
            backtrace->frames[i].regs[j].valid = !((bool) err);
            backtrace->frames[i].regs[j].name  = unw_regname(j);
        }
        i += 1;
    } while (unw_step(&cursor) > 0 && i < LUCET_MAX_BACKTRACE_DEPTH);

    return;
error:
    backtrace->count = 0;
}

void lucet_backtrace_print(struct lucet_backtrace const *backtrace, bool verbose, FILE *file)
{
    fprintf(file, "begin guest backtrace (%lu frames)\n", backtrace->count);
    for (int i = 0; i < backtrace->count; i++) {
        fprintf(file, "  ip=%lx fname=%s sname=%s\n", backtrace->frames[i].ip,
                backtrace->frames[i].file_name, backtrace->frames[i].sym_name);

        if (!verbose) {
            continue;
        }

        // Print two tab-separate cols of registers
        for (int j = 0; j < LUCET_BACKTRACE_N_REGS; j++) {
            if (j % 2 == 0) {
                fprintf(file, "\n    ");
            }
            if (backtrace->frames[i].regs[j].valid) {
                fprintf(file, "%s=%016lx ", backtrace->frames[i].regs[j].name,
                        backtrace->frames[i].regs[j].value);
            } else {
                fprintf(file, "%s=%-16s ", backtrace->frames[i].regs[j].name, "(unknown)");
            }
        }
        fprintf(file, "\n\n");
    }
    fprintf(file, "end backtrace\n");
}

void lucet_backtrace_fatal_handler(struct lucet_instance const *i)
{
    assert(i);
    char                      displaybuf[1024];
    struct lucet_state const *state = lucet_instance_get_state(i);
    int                       res   = lucet_state_display(displaybuf, sizeof(displaybuf), state);
    assert(res > 0);

    fprintf(stderr, "> instance %p had fatal error %s\n", (void *) i, displaybuf);
    struct lucet_backtrace backtrace;

    if (state->tag == lucet_state_fault) {
        lucet_backtrace_create(&backtrace, &state->u.fault.context);
        lucet_backtrace_print(&backtrace, false, stderr);
    }

    // fatal handlers are expected to never return
    abort();
}
