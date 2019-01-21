#include "session_hostcalls.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char hello[] = "hello from sandbox_alloc.c!";

__attribute__((visibility("default"))) int main(void)
{
    session_hostcall_send((unsigned char *) hello, sizeof(hello));

    unsigned char key[] = "X-Sandbox";

    unsigned char *val     = malloc(256);
    size_t         val_len = 256;
    session_hostcall_get_header(key, sizeof(key) - 1, val, &val_len);

    unsigned char output[256] = { 0 };
    size_t        out_len;
    if (val_len > 0) {
        out_len = snprintf((char *) output, sizeof(output), "got sandbox key: %s\n", val);
    } else {
        out_len = snprintf((char *) output, sizeof(output), "sandbox key not found :(\n");
    }

    session_hostcall_send(output, out_len);

    free(val);

    return 0;
}
