#include <assert.h>
#include <err.h>
#include <immintrin.h>
#include <limits.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "lucet_val_private.h"

static int lucet_val_check_u64_bounds(enum lucet_val_type type, uint64_t v64)
{
    uint64_t vmax = 0;
    switch (type) {
    case lucet_val_guest_ptr:
        vmax = UINT32_MAX;
        break;
    case lucet_val_u8:
        vmax = UINT8_MAX;
        break;
    case lucet_val_u16:
        vmax = UINT16_MAX;
        break;
    case lucet_val_u32:
        vmax = UINT32_MAX;
        break;
    case lucet_val_u64:
        vmax = UINT64_MAX;
        break;
    case lucet_val_usize:
        vmax = SIZE_MAX;
        break;
    case lucet_val_c_uchar:
        vmax = UCHAR_MAX;
        break;
    case lucet_val_c_ushort:
        vmax = USHRT_MAX;
        break;
    case lucet_val_c_uint:
        vmax = UINT_MAX;
        break;
    case lucet_val_c_ulong:
        vmax = ULONG_MAX;
        break;
    case lucet_val_c_ulonglong:
        vmax = ULONG_LONG_MAX;
        break;
    case lucet_val_bool:
        vmax = 1;
        break;
    default:
        errx(1, "%s() unexpected lucet_val type", __FUNCTION__);
    }
    if (v64 > vmax) {
        return -1;
    }
    return 0;
}

static int lucet_val_check_i64_bounds(enum lucet_val_type type, int64_t v64)
{
    int64_t vmin = 0;
    int64_t vmax = 0;
    switch (type) {
    case lucet_val_i8:
        vmin = INT8_MIN;
        vmax = INT8_MAX;
        break;
    case lucet_val_i16:
        vmin = INT16_MIN;
        vmax = INT16_MAX;
        break;
    case lucet_val_i32:
        vmin = INT32_MIN;
        vmax = INT32_MAX;
        break;
    case lucet_val_i64:
        vmin = INT64_MIN;
        vmax = INT64_MAX;
        break;
    case lucet_val_isize:
        vmin = -1;
        vmax = SSIZE_MAX;
        break;
    case lucet_val_c_char:
        vmin = CHAR_MIN;
        vmax = CHAR_MAX;
        break;
    case lucet_val_c_short:
        vmin = SHRT_MIN;
        vmax = SHRT_MAX;
        break;
    case lucet_val_c_int:
        vmin = INT_MIN;
        vmax = INT_MAX;
        break;
    case lucet_val_c_long:
        vmin = LONG_MIN;
        vmax = LONG_MAX;
        break;
    case lucet_val_c_longlong:
        vmin = LONG_LONG_MIN;
        vmax = LONG_LONG_MAX;
        break;
    default:
        errx(1, "%s() unexpected lucet_val type", __FUNCTION__);
    }
    if (v64 > vmax || v64 < vmin) {
        return -1;
    }
    return 0;
}

enum lucet_val_register_class lucet_val_register_class(const struct lucet_val *val)
{
    const int val_class = (int) (((unsigned int) val->type >> 16) & 0xffff);
    switch (val_class) {
    case lucet_val_class_as_c_ptr:
    case lucet_val_class_as_u64:
    case lucet_val_class_as_i64:
        return lucet_val_register_class_gp;
    case lucet_val_class_as_f32:
    case lucet_val_class_as_f64:
        return lucet_val_register_class_fp;
    default:
        errx(1, "%s() unexpected lucet_val class", __FUNCTION__);
    }
}

int lucet_val_transmute_to_u64(uint64_t *ret_p, const struct lucet_val *val)
{
    const int val_class = (int) (((unsigned int) val->type >> 16) & 0xffff);
    switch (val_class) {
    case lucet_val_class_as_c_ptr:
        *ret_p = (uint64_t)(uintptr_t) val->inner_val.as_c_ptr;
        return 0;
    case lucet_val_class_as_u64:
        *ret_p = (uint64_t) val->inner_val.as_u64;
        return lucet_val_check_u64_bounds(val->type, val->inner_val.as_u64);
    case lucet_val_class_as_i64:
        *ret_p = (uint64_t) val->inner_val.as_i64;
        return lucet_val_check_i64_bounds(val->type, val->inner_val.as_i64);
    case lucet_val_class_as_f32: {
        _Static_assert(sizeof val->inner_val.as_f32 <= sizeof *ret_p, "cannot transmute");
        *ret_p = 0U;
        memcpy(ret_p, &val->inner_val.as_f32, sizeof val->inner_val.as_f32);
        return 0;
    }
    case lucet_val_class_as_f64: {
        _Static_assert(sizeof val->inner_val.as_f64 == sizeof *ret_p, "cannot transmute");
        memcpy(ret_p, &val->inner_val.as_f64, sizeof val->inner_val.as_f64);
        return 0;
    }
    default:
        errx(1, "%s() unexpected lucet_val class", __FUNCTION__);
    }
}

int lucet_val_transmute_to___m128(__m128 *ret_p, const struct lucet_val *val)
{
    const int val_class = (int) (((unsigned int) val->type >> 16) & 0xffff);
    switch (val_class) {
    case lucet_val_class_as_f32:
        *ret_p = _mm_load_ps1(&val->inner_val.as_f32);
        return 0;
    case lucet_val_class_as_f64:
        *ret_p = _mm_castpd_ps(_mm_load_pd1(&val->inner_val.as_f64));
        return 0;
    default:
        errx(1, "%s() unexpected lucet_val class", __FUNCTION__);
    }
}

union lucet_retval_gp lucet_retval_gp(const struct lucet_untyped_retval *untyped_retval)
{
    union lucet_retval_gp retval_gp;

    _Static_assert(sizeof retval_gp.as_untyped >= sizeof untyped_retval->gp,
                   "retval_gp.as_untyped < untyped_retval.gp");
    memcpy(retval_gp.as_untyped, untyped_retval->gp, sizeof untyped_retval->gp);
    return retval_gp;
}

float lucet_retval_f32(const struct lucet_untyped_retval *untyped_retval)
{
    float retval_f32;
    _mm_storeu_ps(&retval_f32, _mm_loadu_ps((const float *) (const void *) untyped_retval->fp));
    return retval_f32;
}

double lucet_retval_f64(const struct lucet_untyped_retval *untyped_retval)
{
    double retval_f64;
    _mm_storeu_pd(&retval_f64, _mm_loadu_pd((const double *) (const void *) untyped_retval->fp));
    return retval_f64;
}
