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
__attribute__((visibility("default"))) int run_strcmp(const char *s1, const char *s2)
{
    int res = strcmp(s1, s2);
    return res;
}
