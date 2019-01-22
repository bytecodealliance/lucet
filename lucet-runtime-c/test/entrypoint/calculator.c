#include <assert.h>
#include <stdio.h>
#include <string.h>

#include "../helpers.h"
#include "../src/lucet_module_private.h"
#include "../src/lucet_vmctx_private.h"

DEFINE_DEFAULT_HEAP_SPEC;
DEFINE_DEFAULT_GLOBAL_SPEC;
DEFINE_DEFAULT_DATA_SEGMENTS;
DEFINE_DEFAULT_SPARSE_PAGE_DATA;

uint64_t guest_func_add_2(struct lucet_vmctx *ctx, uint64_t arg0, uint64_t arg1)
{
    (void) ctx;
    return arg0 + arg1;
}

uint64_t guest_func_add_10(struct lucet_vmctx *ctx,
                           uint64_t            arg0,
                           uint64_t            arg1,
                           uint64_t            arg2,
                           uint64_t            arg3,
                           uint64_t            arg4,
                           uint64_t            arg5,
                           uint64_t            arg6,
                           uint64_t            arg7,
                           uint64_t            arg8,
                           uint64_t            arg9)
{
    (void) ctx;

    return arg0 + arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8 + arg9;
}

uint64_t guest_func_mul_2(struct lucet_vmctx *ctx, uint64_t arg0, uint64_t arg1)
{
    (void) ctx;
    return arg0 * arg1;
}

float guest_func_add_f32_2(struct lucet_vmctx *ctx, float arg0, float arg1)
{
    (void) ctx;
    return arg0 + arg1;
}

double guest_func_add_f64_2(struct lucet_vmctx *ctx, double arg0, double arg1)
{
    (void) ctx;
    return arg0 + arg1;
}

float guest_func_add_f32_10(struct lucet_vmctx *ctx,
                            float               arg0,
                            float               arg1,
                            float               arg2,
                            float               arg3,
                            float               arg4,
                            float               arg5,
                            float               arg6,
                            float               arg7,
                            float               arg8,
                            float               arg9)
{
    (void) ctx;

    return arg0 + arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8 + arg9;
}

double guest_func_add_f64_10(struct lucet_vmctx *ctx,
                             double              arg0,
                             double              arg1,
                             double              arg2,
                             double              arg3,
                             double              arg4,
                             double              arg5,
                             double              arg6,
                             double              arg7,
                             double              arg8,
                             double              arg9)
{
    (void) ctx;

    return arg0 + arg1 + arg2 + arg3 + arg4 + arg5 + arg6 + arg7 + arg8 + arg9;
}

double guest_func_add_mixed_20(struct lucet_vmctx *ctx,
                               double              arg0,
                               uint8_t             arg1,
                               float               arg2,
                               double              arg3,
                               uint16_t            arg4,
                               float               arg5,
                               double              arg6,
                               uint32_t            arg7,
                               float               arg8,
                               double              arg9,
                               bool                arg10,
                               float               arg11,
                               double              arg12,
                               int                 arg13,
                               float               arg14,
                               double              arg15,
                               long                arg16,
                               float               arg17,
                               double              arg18,
                               long long           arg19)
{
    (void) ctx;

    return (double) arg0 + (double) arg1 + (double) arg2 + (double) arg3 + (double) arg4 +
           (double) arg5 + (double) arg6 + (double) arg7 + (double) arg8 + (double) arg9 +
           (double) arg10 + (double) arg11 + (double) arg12 + (double) arg13 + (double) arg14 +
           (double) arg15 + (double) arg16 + (double) arg17 + (double) arg18 + (double) arg19;
}

struct lucet_table_element guest_table_0 = { .element_type = (uint64_t) 0x0,
                                             .ref =
                                                 (uint64_t)(uintptr_t)(void *) &guest_func_add_2 };

uint64_t guest_table_0_len = sizeof guest_table_0;
