#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "lucet_state.h"
#include "lucet_trap_private.h"

bool lucet_state_runnable(struct lucet_state const *s)
{
    return (s->tag == lucet_state_ready);
}

bool lucet_state_error(struct lucet_state const *s)
{
    return (s->tag == lucet_state_fault || s->tag == lucet_state_terminated);
}

bool lucet_state_fatal(struct lucet_state const *s)
{
    if (s->tag == lucet_state_fault) {
        return s->u.fault.fatal;
    } else {
        return false;
    }
}

static int display_fault(char *str, size_t len, struct lucet_state_fault const *);

int lucet_state_display(char *str, size_t len, struct lucet_state const *s)
{
    switch (s->tag) {
    case lucet_state_ready:
        return snprintf(str, len, "ready");
    case lucet_state_running:
        return snprintf(str, len, "running");
    case lucet_state_fault: {
        size_t orig_len = len;
        int    res;

        res = snprintf(str, len, "fault %s", s->u.fault.fatal ? "FATAL " : "");
        if (res > 0) {
            str += res;
            len -= res;
        } else {
            return res;
        }

        res = display_fault(str, len, &s->u.fault);
        if (res > 0) {
            str += res;
            len -= res;
        } else {
            return res;
        }

        return orig_len - len;
    }
    case lucet_state_terminated: {
        int res = snprintf(str, len, "terminated");
        return res;
    } break;
    default:
        return snprintf(str, len, "<invalid lucet_state tag>");
    }
}

static int display_fault(char *str, size_t len, struct lucet_state_fault const *e)
{
    size_t orig_len = len;
    int    res;

    res = lucet_trapcode_display(str, len, &e->trapcode);
    if (res > 0) {
        str += res;
        len -= res;
    } else {
        return res;
    }

    res = snprintf(str, len, " triggered by %s: ", strsignal(e->signal_info.si_signo));
    if (res > 0) {
        str += res;
        len -= res;
    } else {
        return res;
    }

    res = snprintf(str, len, "code at address %p", (void *) e->rip_addr);
    if (res > 0) {
        str += res;
        len -= res;
    } else {
        return res;
    }

    if (e->rip_addr_details.file_name) {
        res = snprintf(str, len, " (symbol %s:%s)", e->rip_addr_details.file_name,
                       e->rip_addr_details.sym_name ? e->rip_addr_details.sym_name : "<unknown>");
        if (res > 0) {
            str += res;
            len -= res;
        } else {
            return res;
        }
    }

    if (!e->rip_addr_details.module_code_resolvable) {
        res = snprintf(str, len, " (unknown whether in module)");
        if (res > 0) {
            str += res;
            len -= res;
        } else {
            return res;
        }
    } else {
        res = snprintf(str, len, " (%s module code)",
                       e->rip_addr_details.in_module_code ? "inside" : "not inside");
        if (res > 0) {
            str += res;
            len -= res;
        } else {
            return res;
        }
    }

    switch (e->signal_info.si_signo) {
    case SIGSEGV:
    case SIGBUS:
        // We know this is inside the heap guard, because by the time we
        // get here, `lucet_error_verify_trap_safety` will have run and
        // validated it.
        res = snprintf(str, len, " accessed memory at %p (inside heap guard)",
                       e->signal_info.si_addr);
        if (res > 0) {
            str += res;
            len -= res;
        } else {
            return res;
        }
        break;
    }

    return orig_len - len;
}

const char *lucet_state_name(int tag)
{
    switch ((enum lucet_state_tag) tag) {
    case lucet_state_ready:
        return "ready";
    case lucet_state_running:
        return "running";
    case lucet_state_fault:
        return "fault";
    case lucet_state_terminated:
        return "terminated";
    default:
        return "<invalid>";
    }
}
