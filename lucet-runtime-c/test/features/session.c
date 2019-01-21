#include <assert.h>
#include <inttypes.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#include "session.h"

#define OUTPUT_SIZE (1024)

void session_create(struct session *s, const unsigned char *headers)
{
    unsigned char *output = malloc(OUTPUT_SIZE);
    assert(output);

    *s = (struct session){
        .headers      = headers,
        .headers_size = strlen((const char *) headers),

        .output        = output,
        .output_size   = OUTPUT_SIZE,
        .output_cursor = 0,
    };
}

void session_destroy(struct session *s)
{
    assert(s);
    free(s->output);
}

void session_print_output(struct session *s, FILE *f)
{
    assert(s);
    assert(f);
    assert(s->output_cursor < s->output_size);
    // Terminate string
    s->output[s->output_cursor] = 0;
    fprintf(f, "%s", s->output);
}

void session_get_header(struct session const *s, const unsigned char *key, uint32_t key_len,
                        unsigned char *value, uint32_t *value_len)
{
    if (0 == strncmp((const char *) key, (const char *) s->headers, key_len) &&
        (s->headers[key_len] == ':')) {
        const unsigned char *val_start        = &s->headers[key_len + 1];
        uint32_t             remaining_header = s->headers_size - (key_len + 1);
        if (remaining_header < *value_len) {
            *value_len = remaining_header;
        }
        strncpy((char *) value, (const char *) val_start, *value_len);
        return;
    }
    *value_len = 0;
}

void session_send(struct session *s, const unsigned char *chunk, size_t chunk_len)
{
    size_t o = 0;
    for (; (o < chunk_len) && (s->output_cursor + o < s->output_size); o++) {
        if (chunk[o] != 0) {
            s->output[s->output_cursor + o] = chunk[o];
        } else {
            s->output[s->output_cursor + o] = '\n';
        }
    }
    s->output_cursor += o;
    s->output[s->output_cursor] = 0;
}

uint32_t session_stdio_write(struct session *s, int32_t fd, const char *chunk, size_t chunk_len)
{
    char *output = malloc(OUTPUT_SIZE);
    assert(output);

    int len = snprintf(output, OUTPUT_SIZE, "stdio %d > ", fd);
    assert(len > 0);
    memcpy(&output[len], chunk, chunk_len);
    uint32_t written = chunk_len + len;
    session_send(s, (unsigned char *) output, written);
    return written;
}
