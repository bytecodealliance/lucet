#ifndef LUCET_GLOBALS_PRIVATE_H
#define LUCET_GLOBALS_PRIVATE_H

#include <stdbool.h>
#include <stdint.h>

// Each module's shared library exports a symbol `lucet_globals_spec` that is
// first a `struct lucet_globals_spec` followed immediately in memory by the
// number of `lucet_globals_descriptor` specified in the spec. These values
// correspond to the globals of the module, and are ordered by their wasm global
// index.
struct lucet_globals_spec {
    uint64_t num_globals;
};

struct lucet_globals_descriptor {
    // Flags described by defines below.
    uint64_t flags;
    // internal definitions have the following initial value. imports initial
    // values are defined elsewhere.
    int64_t initial_value;
    // If valid name flag is set, this should be non-null. Import names have the
    // module and name separated by "::". Internal definition names have no
    // separator.
    const char *name;
};
// Each global is either an internal definition or an import. Internal
// definitions always have an initial value provided in the descriptor. The
// initial value of an import is given by the environment (if possible).
#define LUCET_GLOBALS_DESCRIPTOR_FLAG_IMPORT (1 << 0)
// Imports always have a name. Internal definitions sometimes have a name (if
// they are marked as "export").
#define LUCET_GLOBALS_DESCRIPTOR_FLAG_VALID_NAME (1 << 1)

/**
 * Validate that the lucet_globals_spec can be implemented. Returns true if
 * possible, false otherwise.
 */
bool lucet_globals_validate(struct lucet_globals_spec const *spec);

/**
 * Initialize the globals. Precondition: validate returned true
 */
void lucet_globals_initialize(struct lucet_globals_spec const *spec, int64_t *globals);

#endif // LUCET_GLOBALS_PRIVATE_H
