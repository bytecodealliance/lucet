
#include <stddef.h>

// Rather than have a dependency on a libc with the right
// wasi snapshot, lets just import the single func we are
// going to call:
extern int args_sizes_get(size_t* a, size_t* b)
    __attribute__((import_module("wasi_snapshot_preview1")));

int sum_of_arg_sizes(void) {
    size_t a = 0;
    size_t b = 0;
    int res = args_sizes_get(&a, &b);
    if (res != 0) {
        return 0;
    }

    return (int) a + (int) b;
}
