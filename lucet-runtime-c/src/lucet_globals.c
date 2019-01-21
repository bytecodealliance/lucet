#include "lucet_globals_private.h"

bool lucet_globals_validate(struct lucet_globals_spec const *spec)
{
    uint64_t num = spec->num_globals;
    for (int i = 0; i < num; i++) {
        struct lucet_globals_descriptor const *descriptor =
            (struct lucet_globals_descriptor *) ((uintptr_t) spec +
                                                 sizeof(struct lucet_globals_spec) +
                                                 (i * sizeof(struct lucet_globals_descriptor)));
        if (descriptor->flags & LUCET_GLOBALS_DESCRIPTOR_FLAG_IMPORT) {
            return false;
        }
    }
    return true;
}

void lucet_globals_initialize(struct lucet_globals_spec const *spec, int64_t *globals)
{
    uint64_t num = spec->num_globals;
    for (int i = 0; i < num; i++) {
        struct lucet_globals_descriptor const *descriptor =
            (struct lucet_globals_descriptor *) ((uintptr_t) spec +
                                                 sizeof(struct lucet_globals_spec) +
                                                 (i * sizeof(struct lucet_globals_descriptor)));
        // Determined that the global is not an import by the validation check,
        // which is a precondition for this call.
        globals[i] = descriptor->initial_value;
        // They also might have an export name given by
        // descriptor->name, but we aren't using it for anything right
        // now.
    }
}
