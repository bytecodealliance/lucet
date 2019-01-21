#include <assert.h>
#include <errno.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char hello[] = "hello from musl_alloc.c!";

// Must observe pointers from a different compilation unit so that Clang doesnt
// "helpfully" erase mallocs/frees
void observe(void *ptr);

static bool failed = false;
static void fail(const char *reason)
{
    failed = true;
    fputs(reason, stderr);
}

__attribute__((visibility("default"))) int main(void)
{
    char *heap_str = malloc(256);
    if (heap_str != NULL) {
        snprintf(heap_str, 256, "this is a string located in the heap: %s\n", hello);
        puts(heap_str);
    } else {
        fail("malloc(256) returned NULL");
    }

    // The following tests are based on the glibc test suite "tst-malloc.c":
    void *p    = malloc(-1);
    int   save = errno;
    observe(p);
    if (p != NULL) {
        fail("malloc(-1) succeeded");
    }

    if ((p == NULL) && save != ENOMEM) {
        fail("errno not set correctly after malloc(-1)");
    }

    p = malloc(10);
    observe(p);
    if (p == NULL) {
        fail("malloc(10) returned NULL");
    }

    p = realloc(p, 0);
    observe(p);
    if (p != NULL) {
        fail("realloc(p, 0) returned non-null");
    }

    p = malloc((128 * 1024) - 28);
    observe(p);
    if (p == NULL) {
        fail("malloc(128KiB - 28) returned NULL");
    }
    free(p);

    p = malloc(128 * 1024);
    observe(p);
    if (p == NULL) {
        fail("malloc(128KiB) returned NULL");
    }

    p    = malloc(512 * 1024 * 1024);
    save = errno;
    observe(p);
    if (p != NULL) {
        fail(
            "malloc(512M) returned non-null: "
            "we don't expect our liblucet-runtime-c to ever provide that much memory");
    }
    if (p == NULL && save != ENOMEM) {
        fail("errno not set correctly after malloc(512M)");
    }

    p = malloc(-512 * 1024);
    observe(p);
    if (p != NULL) {
        fail("malloc(-512K) returned non-null");
    }

    free(heap_str);

    // Make sure the runtime can tell there was a failure by checking state
    assert(!failed);

    return 0;
}
