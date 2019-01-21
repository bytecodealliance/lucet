/*
 * This file implements a WASM guest program that has essentially the
 * same functionality as guest.c but allows for injecting faults for testing
 * purposes. The resulting shared object can be loaded via a Varnish mgt_param.
 */

/*
 * Cause a fault in the WASM guest
 */
__attribute__((visibility("default"))) void wasm_fault(void)
{
    // char *oob = (char *) -1; // BUG: llvm 6.0.0 encodes this address
    // incorrectly and makes an invalid wasm file. So, I am switching to an
    // address that is out-of-bounds in liblucet-runtime-c but can be represented in less
    // than 32 bits, but is beyond the end of the guard pages
    char *oob = (char *) (64 * 1000 * 1000);
    *oob      = 'x';
}

/*
 * Cause a fault in the host
 */
extern void                                 hostcall_host_fault(void);
__attribute__((visibility("default"))) void host_fault(void)
{
    hostcall_host_fault();
}

#include "guest.c"
