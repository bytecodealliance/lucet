
#ifndef GLOBALS_H
#define GLOBALS_H

#include "vm.h"

// Each global is either an internal definition (which has an initial value), or
// an import, whose initial value is determined elsewhere. The 0th bit is set
// when it is an import.
#define GLOBALS_FLAG_IMPORT (1 << 0)
// Each global that is an import, and some globals that are internally defined,
// have a name. This bit is set when the char* name field points to a valid
// name.
#define GLOBALS_FLAG_VALID_NAME (1 << 1)

struct global_description {
    uint64_t flags;
    int64_t  initial_value;
    char *   name;
};

struct global_table {
    uint64_t num_descriptors;
};

void initialize_globals(struct VM *, struct global_table const *);

#endif // GLOBALS_H
