/*
 * WASM guest program.
 *
 * Note that faults are injected (for testing) via a separate program defined in
 * fault_guest.c.
 */

#include <stdint.h>
#include <string.h>

/*
 * We cant take return values from a wasm guest, so we need to wrap
 * strcmp up in a function that takes a pointer for the return value.
 */
int64_t run_strcmp(const char *s1, const char *s2)
{
    int res = strcmp(s1, s2);
    return (int64_t) res;
}

/*
 * This file implements a WASM guest program that has essentially the
 * same functionality as guest.c but allows for injecting faults for testing
 * purposes. The resulting shared object can be loaded via a Varnish mgt_param.
 */

/*
 * Cause a fault in the WASM guest
 */
void wasm_fault(void)
{
    // Create an out-of-bounds pointer, and write to it
    char *oob = (char *) -1;
    *oob      = 'x';
}

/*
 * Cause a fault in the host
 */
extern void hostcall_host_fault(void);
void        host_fault(void)
{
    hostcall_host_fault();
}
