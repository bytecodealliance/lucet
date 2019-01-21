#ifndef LUCET_LIBC_H
#define LUCET_LIBC_H

/* This header is only for use by guest (WASM) code. It exports the WASM ABI
 * version of the lucet_libc hostcalls. */

#include <stddef.h>
#include <stdint.h>

/**
 * Hostcall for implementing stdio
 */
void lucet_libc_stdio(int fd, const char* buf, size_t len);

/**
 * Hostcall for implementing abort()
 */
_Noreturn void lucet_libc_syscall_abort(void);

/**
 * Hostcall for implementing _Exit()
 */
_Noreturn void lucet_libc_syscall_exit(int32_t code);


#endif // LUCET_LIBC_H
