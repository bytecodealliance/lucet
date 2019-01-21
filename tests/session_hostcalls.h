#ifndef SESSION_HOSTCALLS_H
#define SESSION_HOSTCALLS_H

#include <stddef.h>
#include <stdint.h>

// Syscalls: We define these application-specific syscalls for
// the "session" tests in this file. Their implementations get hooked up to the
// session.h interface.

void session_hostcall_get_header(const unsigned char *key, size_t key_len, unsigned char *value,
                                 size_t *value_len);
void session_hostcall_send(const unsigned char *chunk, size_t chunk_len);

#endif // SESSION_HOSTCALLS_H
