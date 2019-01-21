#ifndef LUCET_TRAP_H
#define LUCET_TRAP_H 1

#include <stdint.h>

#include "lucet_export.h"

enum lucet_trapcode_type {
    lucet_trapcode_stack_overflow            = 0,
    lucet_trapcode_heap_oob                  = 1,
    lucet_trapcode_oob                       = 2,
    lucet_trapcode_indirect_call_to_null     = 3,
    lucet_trapcode_bad_signature             = 4,
    lucet_trapcode_integer_overflow          = 5,
    lucet_trapcode_integer_div_by_zero       = 6,
    lucet_trapcode_bad_conversion_to_integer = 7,
    lucet_trapcode_interrupt                 = 8,
    lucet_trapcode_table_out_of_bounds       = 9,
    lucet_trapcode_user                      = UINT16_MAX - 1,
    lucet_trapcode_unknown                   = UINT16_MAX,
};

struct lucet_trapcode {
    enum lucet_trapcode_type code;
    uint16_t                 tag;
};

const char *lucet_trapcode_type_string(int trapcode) EXPORTED;

#endif
