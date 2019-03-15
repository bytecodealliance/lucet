
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

// The WASI implementation of assert pulls facilities for in printing to stderr
// and aborting. This is lighter weight for a unit test
static void assert(bool v) {
    if (!v) {
        __builtin_unreachable();
    }
}

void create_and_memset(int init_as, size_t size, char **ptr_outval)
{
    char *area = malloc(size);
    assert(area);
    memset(area, init_as, size);
    *ptr_outval = area;
}

void increment_ptr(char *ptr)
{
    char val = *ptr;
    *ptr     = val + 1;
}
