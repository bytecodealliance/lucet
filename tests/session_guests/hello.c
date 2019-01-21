#include "session_hostcalls.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char hello[] = "hello from sandbox_hello.c!";

// The sandbox entry point is `main`, this function will be called
// when the sandbox instance is run.
// The `session_hostcall` defs will handle any interaction with the host.
__attribute__((visibility("default"))) int main(void)
{
    session_hostcall_send((unsigned char *) hello, sizeof(hello));

    unsigned char key[] = "X-Sandbox";

    unsigned char val[256] = { 0 };
    size_t        val_len  = sizeof(val);
    session_hostcall_get_header(key, sizeof(key) - 1, val, &val_len);

    unsigned char output[256] = { 0 };
    size_t        out_len;
    if (val_len > 0) {
        out_len = snprintf((char *) output, sizeof(output), "got sandbox key: %s\n", val);
    } else {
        out_len = snprintf((char *) output, sizeof(output), "sandbox key not found :(\n");
    }

    session_hostcall_send(output, out_len);

    // Exit with a non-zero code
    if (val[0] == '3') {
        const char msg[] = "sandbox is going to exit with -1\n";
        session_hostcall_send((unsigned char *) msg, strlen(msg));
        exit(-1);
    }

    // Deliberately write to some invalid address to check if the sandbox
    // catches the invalid memory access:
    if (val[0] == '4') {
        const char msg[] = "sandbox is going to access invalid memory\n";
        session_hostcall_send((unsigned char *) msg, strlen(msg));
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Warray-bounds"
        ((char *) hello)[(1024 * 1024) + 1] = '\n';
#pragma clang diagnostic pop
    }

    // We use libc to provide an exit status.
    exit(0);
    // TODO: make the `main` return value get passed to exit when main returns
    return 0;
}
