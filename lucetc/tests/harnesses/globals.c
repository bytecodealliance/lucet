
#include "globals.h"
#include <assert.h>
#include <err.h>

void initialize_globals(struct VM *vm, struct global_table const *tbl)
{
    uint64_t num = tbl->num_descriptors;
    assert(num < GLOBALS_SIZE);

    for (int i = 0; i < num; i++) {
        struct global_description *descriptor =
            (struct global_description *) ((uintptr_t) tbl + sizeof(uint64_t) +
                                           (i * sizeof(struct global_description)));
        if (!(descriptor->flags & GLOBALS_FLAG_IMPORT)) {
            vm->globals[i] = descriptor->initial_value;
        } else {
            errx(1, "%s() unit testing of imports is not supported", __FUNCTION__);
        }
    }
}
