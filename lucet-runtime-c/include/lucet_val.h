#ifndef LUCET_VAL_H
#define LUCET_VAL_H 1

/*
 * Typed values
 *
 * `struct lucet_val` represents a typed value, used in arguments lists.
 * Such arguments can be built with the `LUCET_VAL_*` convenience macros.
 *
 * A guest function call with these arguments eventually returns a
 * `struct lucet_untyped_retval` value, that can be converted to a
 * native type with the `LUCET_UNTYPED_RETVAL_TO_*` macros.
 *
 * Usage:
 *
 * inst = lucet_instance_run(inst, "add_2", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));
 * state = lucet_instance_get_state(inst);
 * uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);
 */

#include <sys/types.h>

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "lucet_export.h"

/*
 * Note on the value associated with each type:
 * the most significant bits represent the "class" of the type
 * (0: void, 1: a C pointer, 2: something unsigned that fits in 64 bits,
 *  3: something signed that fits in 64 bits, 4: f32, 5: f64).
 * The remain bits can be anything as long as it is unique.
 */
enum lucet_val_type {
    lucet_val_void        = ((uint32_t) 0 << 16) | 0x0001,
    lucet_val_c_ptr       = ((uint32_t) 1 << 16) | 0x0100,
    lucet_val_guest_ptr   = ((uint32_t) 2 << 16) | 0x0101,
    lucet_val_u8          = ((uint32_t) 2 << 16) | 0x0201,
    lucet_val_u16         = ((uint32_t) 2 << 16) | 0x0202,
    lucet_val_u32         = ((uint32_t) 2 << 16) | 0x0203,
    lucet_val_u64         = ((uint32_t) 2 << 16) | 0x0204,
    lucet_val_i8          = ((uint32_t) 3 << 16) | 0x0300,
    lucet_val_i16         = ((uint32_t) 3 << 16) | 0x0301,
    lucet_val_i32         = ((uint32_t) 3 << 16) | 0x0302,
    lucet_val_i64         = ((uint32_t) 3 << 16) | 0x0303,
    lucet_val_usize       = ((uint32_t) 2 << 16) | 0x0400,
    lucet_val_isize       = ((uint32_t) 3 << 16) | 0x0401,
    lucet_val_c_uchar     = ((uint32_t) 2 << 16) | 0x0500,
    lucet_val_c_ushort    = ((uint32_t) 2 << 16) | 0x0501,
    lucet_val_c_uint      = ((uint32_t) 2 << 16) | 0x0502,
    lucet_val_c_ulong     = ((uint32_t) 2 << 16) | 0x0503,
    lucet_val_c_ulonglong = ((uint32_t) 2 << 16) | 0x0504,
    lucet_val_c_char      = ((uint32_t) 3 << 16) | 0x0600,
    lucet_val_c_short     = ((uint32_t) 3 << 16) | 0x0601,
    lucet_val_c_int       = ((uint32_t) 3 << 16) | 0x0602,
    lucet_val_c_long      = ((uint32_t) 3 << 16) | 0x0603,
    lucet_val_c_longlong  = ((uint32_t) 3 << 16) | 0x0604,
    lucet_val_bool        = ((uint32_t) 2 << 16) | 0x0700,
    lucet_val_f32         = ((uint32_t) 4 << 16) | 0x0800,
    lucet_val_f64         = ((uint32_t) 5 << 16) | 0x0801
};

union lucet_val_inner_val {
    void *   as_c_ptr; // ((uint32_t) 1 << 16)
    uint64_t as_u64;   // ((uint32_t) 2 << 16)
    int64_t  as_i64;   // ((uint32_t) 3 << 16)
    float    as_f32;   // ((uint32_t) 4 << 16)
    double   as_f64;   // ((uint32_t) 5 << 16)
};

// A typed value, typically used to build arguments lists
struct lucet_val {
    enum lucet_val_type       type;
    union lucet_val_inner_val inner_val;
};

// Creates an lucet_val value from the given type

#define LUCET_VAL_VOID \
    ((struct lucet_val[1]){ { .type = lucet_val_void, .inner_val.as_u64 = 0 } }[0])
#define LUCET_VAL_T(T, C, X) \
    ((struct lucet_val[1]){ { .type = lucet_val_##T, .inner_val.C = (X) } }[0])

#define LUCET_VAL_C_PTR(X) LUCET_VAL_T(c_ptr, as_c_ptr, X)
#define LUCET_VAL_GUEST_PTR(X) LUCET_VAL_T(guest_ptr, as_u64, X)

#define LUCET_VAL_U8(X) LUCET_VAL_T(u8, as_u64, X)
#define LUCET_VAL_U16(X) LUCET_VAL_T(u16, as_u64, X)
#define LUCET_VAL_U32(X) LUCET_VAL_T(u32, as_u64, X)
#define LUCET_VAL_U64(X) LUCET_VAL_T(u64, as_u64, X)

#define LUCET_VAL_I8(X) LUCET_VAL_T(i8, as_i64, X)
#define LUCET_VAL_I16(X) LUCET_VAL_T(i16, as_i64, X)
#define LUCET_VAL_I32(X) LUCET_VAL_T(i32, as_i64, X)
#define LUCET_VAL_I64(X) LUCET_VAL_T(i64, as_i64, X)

#define LUCET_VAL_USIZE(X) LUCET_VAL_T(usize, as_u64, X)
#define LUCET_VAL_ISIZE(X) LUCET_VAL_T(isize, as_i64, X)

#define LUCET_VAL_C_UCHAR(X) LUCET_VAL_T(c_uchar, as_u64, X)
#define LUCET_VAL_C_USHORT(X) LUCET_VAL_T(c_ushort, as_u64, X)
#define LUCET_VAL_C_UINT(X) LUCET_VAL_T(c_uint, as_u64, X)
#define LUCET_VAL_C_ULONG(X) LUCET_VAL_T(c_ulong, as_u64 X)
#define LUCET_VAL_C_ULONGLONG(X) LUCET_VAL_T(c_ulonglong, as_u64, X)

#define LUCET_VAL_C_CHAR(X) LUCET_VAL_T(c_char, as_i64, X)
#define LUCET_VAL_C_SHORT(X) LUCET_VAL_T(c_short, as_i64, X)
#define LUCET_VAL_C_INT(X) LUCET_VAL_T(c_int, as_i64, X)
#define LUCET_VAL_C_LONG(X) LUCET_VAL_T(c_long, as_i64, X)
#define LUCET_VAL_C_LONGLONG(X) LUCET_VAL_T(c_longlong, as_i64, X)

#define LUCET_VAL_BOOL(X) LUCET_VAL_T(bool, as_u64, X)

#define LUCET_VAL_F32(X) LUCET_VAL_T(f32, as_f32, X)
#define LUCET_VAL_F64(X) LUCET_VAL_T(f64, as_f64, X)

// Converts an lucet_val value to the given type

#define LUCET_VAL_TO_T(T, C, V) ((T)((V).inner_val.C))

#define LUCET_VAL_TO_C_PTR(X) LUCET_VAL_TO_T(void *, as_c_ptr, X)
#define LUCET_VAL_TO_GUEST_PTR(X) LUCET_VAL_TO_T(guest_ptr_t, as_u64, X)

#define LUCET_VAL_TO_U8(X) LUCET_VAL_TO_T(uint8_t, as_u64, X)
#define LUCET_VAL_TO_U16(X) LUCET_VAL_TO_T(uint16_t, as_u64, X)
#define LUCET_VAL_TO_U32(X) LUCET_VAL_TO_T(uint32_t, as_u64, X)
#define LUCET_VAL_TO_U64(X) LUCET_VAL_TO_T(uint64_t, as_u64, X)

#define LUCET_VAL_TO_I8(X) LUCET_VAL_TO_T(int8_t, as_i64, X)
#define LUCET_VAL_TO_I16(X) LUCET_VAL_TO_T(int16_t, as_i64, X)
#define LUCET_VAL_TO_I32(X) LUCET_VAL_TO_T(int32_t, as_i64, X)
#define LUCET_VAL_TO_I64(X) LUCET_VAL_TO_T(int64_t, as_i64, X)

#define LUCET_VAL_TO_USIZE(X) LUCET_VAL_TO_T(size_t, as_u64, X)
#define LUCET_VAL_TO_ISIZE(X) LUCET_VAL_TO_T(ssize_t, as_i64, X)

#define LUCET_VAL_TO_C_UCHAR(X) LUCET_VAL_TO_T(unsigned char, as_u64, X)
#define LUCET_VAL_TO_C_USHORT(X) LUCET_VAL_TO_T(unsigned short, as_u64, X)
#define LUCET_VAL_TO_C_UINT(X) LUCET_VAL_TO_T(unsigned int, as_u64, X)
#define LUCET_VAL_TO_C_ULONG(X) LUCET_VAL_TO_T(unsigned long, as_u64 X)
#define LUCET_VAL_TO_C_ULONGLONG(X) LUCET_VAL_TO_T(unsigned long long, as_u64, X)

#define LUCET_VAL_TO_C_CHAR(X) LUCET_VAL_TO_T(char, as_i64, X)
#define LUCET_VAL_TO_C_SHORT(X) LUCET_VAL_TO_T(short, as_i64, X)
#define LUCET_VAL_TO_C_INT(X) LUCET_VAL_TO_T(int, as_i64, X)
#define LUCET_VAL_TO_C_LONG(X) LUCET_VAL_TO_T(long, as_i64, X)
#define LUCET_VAL_TO_C_LONGLONG(X) LUCET_VAL_TO_T(long long, as_i64, X)

#define LUCET_VAL_TO_BOOL(X) LUCET_VAL_TO_T(bool, as_u64, X)

#define LUCET_VAL_TO_F32(X) LUCET_VAL_TO_T(float, as_f32, X)
#define LUCET_VAL_TO_F64(X) LUCET_VAL_TO_T(double, as_f64, X)

// Return values

// An untyped value, returned by guest function calls
struct lucet_untyped_retval {
    unsigned char fp[16];
    unsigned char gp[8];
};

union lucet_retval_gp {
    unsigned char as_untyped[8];
    void *        as_c_ptr;
    uint64_t      as_u64;
    int64_t       as_i64;
};

// Converts an untyped return value to the given type

#define LUCET_UNTYPED_RETVAL_TO_GP_T(T, C, X) ((T) lucet_retval_gp(&(X)).C)
#define LUCET_UNTYPED_RETVAL_TO_C_PTR(X) LUCET_UNTYPED_RETVAL_TO_GP_T(void *, as_c_ptr, &(X))

#define LUCET_UNTYPED_RETVAL_TO_GUEST_PTR(X) LUCET_UNTYPED_RETVAL_TO_GP_T(guest_ptr_t, as_u64, X)

#define LUCET_UNTYPED_RETVAL_TO_U8(X) LUCET_UNTYPED_RETVAL_TO_GP_T(uint8_t, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_U16(X) LUCET_UNTYPED_RETVAL_TO_GP_T(uint16_t, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_U32(X) LUCET_UNTYPED_RETVAL_TO_GP_T(uint32_t, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_U64(X) LUCET_UNTYPED_RETVAL_TO_GP_T(uint64_t, as_u64, X)

#define LUCET_UNTYPED_RETVAL_TO_I8(X) LUCET_UNTYPED_RETVAL_TO_GP_T(int8_t, as_i64, X)
#define LUCET_UNTYPED_RETVAL_TO_I16(X) LUCET_UNTYPED_RETVAL_TO_GP_T(int16_t, as_i64, X)
#define LUCET_UNTYPED_RETVAL_TO_I32(X) LUCET_UNTYPED_RETVAL_TO_GP_T(int32_t, as_i64, X)
#define LUCET_UNTYPED_RETVAL_TO_I64(X) LUCET_UNTYPED_RETVAL_TO_GP_T(int64_t, as_i64, X)

#define LUCET_UNTYPED_RETVAL_TO_USIZE(X) LUCET_UNTYPED_RETVAL_TO_GP_T(size_t, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_ISIZE(X) LUCET_UNTYPED_RETVAL_TO_GP_T(ssize_t, as_i64, X)

#define LUCET_UNTYPED_RETVAL_TO_C_UCHAR(X) LUCET_UNTYPED_RETVAL_TO_GP_T(unsigned char, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_C_USHORT(X) LUCET_UNTYPED_RETVAL_TO_GP_T(unsigned short, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_C_UINT(X) LUCET_UNTYPED_RETVAL_TO_GP_T(unsigned int, as_u64, X)
#define LUCET_UNTYPED_RETVAL_TO_C_ULONG(X) LUCET_UNTYPED_RETVAL_TO_GP_T(unsigned long, as_u64 X)
#define LUCET_UNTYPED_RETVAL_TO_C_ULONGLONG(X) \
    LUCET_UNTYPED_RETVAL_TO_GP_T(unsigned long long, as_u64, X)

#define LUCET_UNTYPED_RETVAL_TO_C_CHAR(X) LUCET_UNTYPED_RETVAL_TO_GP_T(char, as_i64, X)
#define LUCET_UNTYPED_RETVAL_TO_C_SHORT(X) LUCET_UNTYPED_RETVAL_TO_GP_T(short, as_i64, X)
#define LUCET_UNTYPED_RETVAL_TO_C_INT(X) LUCET_UNTYPED_RETVAL_TO_GP_T(int, as_i64, X)
#define LUCET_UNTYPED_RETVAL_TO_C_LONG(X) LUCET_UNTYPED_RETVAL_TO_GP_T(long, as_i64 X)
#define LUCET_UNTYPED_RETVAL_TO_C_LONGLONG(X) LUCET_UNTYPED_RETVAL_TO_GP_T(long long, as_i64, X)

#define LUCET_UNTYPED_RETVAL_TO_BOOL(X) LUCET_UNTYPED_RETVAL_TO_GP_T(bool, as_u64, X)

#define LUCET_UNTYPED_RETVAL_TO_F32(X) lucet_retval_f32(&(X))
#define LUCET_UNTYPED_RETVAL_TO_F64(X) lucet_retval_f64(&(X))

union lucet_retval_gp lucet_retval_gp(const struct lucet_untyped_retval *untyped_retval) EXPORTED;
float                 lucet_retval_f32(const struct lucet_untyped_retval *untyped_retval) EXPORTED;
double                lucet_retval_f64(const struct lucet_untyped_retval *untyped_retval) EXPORTED;

#endif
