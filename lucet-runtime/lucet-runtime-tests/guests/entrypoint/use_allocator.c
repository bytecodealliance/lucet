
#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

__attribute__((visibility("default"))) void create_and_memset(int init_as, size_t size,
                                                              char **ptr_outval)
{
    char *area = malloc(size);
    assert(area);
    memset(area, init_as, size);
    *ptr_outval = area;
}

__attribute__((visibility("default"))) void increment_ptr(char *ptr)
{
    char val = *ptr;
    *ptr     = val + 1;
}
