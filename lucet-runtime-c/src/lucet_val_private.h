#ifndef LUCET_VAL_PRIVATE_H
#define LUCET_VAL_PRIVATE_H 1

#include <immintrin.h>
#include <stdint.h>

#include "lucet_val.h"

// A value class, i.e. what member of the lucet_val_inner_val union is correct for
// the value
enum lucet_val_class {
    lucet_val_class_void     = 0,
    lucet_val_class_as_c_ptr = 1,
    lucet_val_class_as_u64   = 2,
    lucet_val_class_as_i64   = 3,
    lucet_val_class_as_f32   = 4,
    lucet_val_class_as_f64   = 5,
};

// What register type a value should end into
enum lucet_val_register_class {
    lucet_val_register_class_unknown,
    lucet_val_register_class_gp, // general-purpose registers
    lucet_val_register_class_fp  // floating-point registers
};

// Returns what kind of register should be used to store the given value type
enum lucet_val_register_class lucet_val_register_class(const struct lucet_val *val);

// Transmutes a value that fits in 64 bits into a uint64_t
// Returns 0 on success, -1 on overflow.
int lucet_val_transmute_to_u64(uint64_t *ret_p, const struct lucet_val *val);

// Transmutes a floating-point value into a __m128 value
// Returns 0 on success, -1 for invalid values.
int lucet_val_transmute_to___m128(__m128 *ret_p, const struct lucet_val *val);

#endif
