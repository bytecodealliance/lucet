
#ifndef SESSION_H
#define SESSION_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>

#include "lucet_libc.h"

struct session {
    struct lucet_libc libc;

    const unsigned char *headers;
    size_t               headers_size;

    unsigned char *output;
    size_t         output_size;
    size_t         output_cursor;
};

// Create a session, given some headers
void session_create(struct session *, const unsigned char *headers);

// Print all of the things written to the session by `session_send` calls.
void session_print_output(struct session *, FILE *);

// Free memory used by a session
void session_destroy(struct session *);

// API for use by host calls:
void     session_get_header(struct session const *s, const unsigned char *key, uint32_t key_len,
                            unsigned char *value, uint32_t *value_len);
void     session_send(struct session *s, const unsigned char *chunk, size_t chunk_len);
uint32_t session_stdio_write(struct session *s, int32_t fd, const char *chunk, size_t chunk_len);

#endif // SESSION_H
