#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <sys/types.h>

#include "lucet.h"
#include "lucet_wasi.h"
#include "sightglass.h"
#include "wrapper.h"

#define xstr(x) str(x)
#define str(x) #x

typedef struct LucetCtx_ {
    struct lucet_dl_module *mod;
    struct lucet_instance * inst;
    struct lucet_region *   region;
    uint8_t *               heap;
    guest_ptr_t             ctx_p;
} LucetCtx;

static LucetCtx lucet_setup(void)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(xstr(WASM_MODULE), &mod));
    struct lucet_region *region;
    ASSERT_OK(lucet_test_region_create(1, NULL, &region));
    struct lucet_wasi_ctx *wasi_ctx = lucet_wasi_ctx_create();
    struct lucet_instance *inst;
    ASSERT_OK(lucet_region_new_instance_with_wasi_ctx(region, mod, wasi_ctx, &inst));

    uint8_t *heap = lucet_instance_heap(inst);
    uint32_t newpage_start;
    ASSERT_OK(lucet_instance_grow_heap(inst, 1, &newpage_start));
    guest_ptr_t ctx_p = newpage_start * LUCET_WASM_PAGE_SIZE;

    LucetCtx lucet_ctx = {
        .mod = mod, .region = region, .inst = inst, .heap = heap, .ctx_p = ctx_p
    };
    return lucet_ctx;
}

#define LUCET_SETUP lucet_ctx = lucet_setup()

static void lucet_teardown(LucetCtx *lucet_ctx)
{
    lucet_instance_release(lucet_ctx->inst);
    lucet_dl_module_release(lucet_ctx->mod);
    lucet_region_release(lucet_ctx->region);
}

#define LUCET_TEARDOWN lucet_teardown(&lucet_ctx)

TestsConfig tests_config = { .global_setup    = NULL,
                             .global_teardown = NULL,
                             .version         = TEST_ABI_VERSION };

static LucetCtx lucet_ctx;

static void setup_wrapper(const char *name, void *global_ctx_, void **ctx_p)
{
    (void) global_ctx_;
    ASSERT_OK(lucet_instance_run(
        lucet_ctx.inst, name, 2,
        (struct lucet_val[]){ LUCET_VAL_GUEST_PTR(0), LUCET_VAL_GUEST_PTR(lucet_ctx.ctx_p) },
        NULL));
    *ctx_p = (void *) (uintptr_t) * (guest_ptr_t *) &lucet_ctx.heap[lucet_ctx.ctx_p];
}

#define SETUP(NAME)                                        \
    void NAME##_setup(void *global_ctx_, void **ctx_p)     \
    {                                                      \
        LUCET_SETUP;                                       \
        setup_wrapper(#NAME "_setup", global_ctx_, ctx_p); \
    }

#define SETUP_NOWRAP(NAME)                             \
    void NAME##_setup(void *global_ctx_, void **ctx_p) \
    {                                                  \
        (void) global_ctx_;                            \
        (void) ctx_p;                                  \
        LUCET_SETUP;                                   \
    }

static void body_wrapper(const char *name, void *ctx)
{
    lucet_instance_run(lucet_ctx.inst, name, 1,
                       (struct lucet_val[]){ LUCET_VAL_GUEST_PTR((guest_ptr_t)(uintptr_t) ctx) },
                       NULL);
}

#define BODY(NAME) \
    void NAME##_body(void *ctx) { body_wrapper(#NAME "_body", ctx); }

static void teardown_wrapper(const char *name, void *ctx)
{
    lucet_instance_run(lucet_ctx.inst, name, 1,
                       (struct lucet_val[]){ LUCET_VAL_GUEST_PTR((guest_ptr_t)(uintptr_t) ctx) },
                       NULL);
}

#define TEARDOWN(NAME)                            \
    void NAME##_teardown(void *ctx)               \
    {                                             \
        teardown_wrapper(#NAME "_teardown", ctx); \
        LUCET_TEARDOWN;                           \
    }

#define TEARDOWN_NOWRAP(NAME)       \
    void NAME##_teardown(void *ctx) \
    {                               \
        (void) ctx;                 \
        LUCET_TEARDOWN;             \
    }

SETUP(ackermann)
BODY(ackermann)
TEARDOWN_NOWRAP(ackermann);

SETUP(fib2)
BODY(fib2)
TEARDOWN_NOWRAP(fib2)

SETUP_NOWRAP(ed25519)
BODY(ed25519)
TEARDOWN_NOWRAP(ed25519)

SETUP_NOWRAP(gimli)
BODY(gimli)
TEARDOWN_NOWRAP(gimli)

SETUP_NOWRAP(keccak)
BODY(keccak)
TEARDOWN_NOWRAP(keccak)

SETUP_NOWRAP(base64)
BODY(base64)
TEARDOWN_NOWRAP(base64)

SETUP(heapsort)
BODY(heapsort)
TEARDOWN_NOWRAP(heapsort)

SETUP(matrix2)
BODY(matrix2)
TEARDOWN(matrix2)

SETUP(matrix)
BODY(matrix)
TEARDOWN(matrix)

SETUP_NOWRAP(minicsv)
BODY(minicsv)
TEARDOWN_NOWRAP(minicsv)

SETUP(nestedloop3)
BODY(nestedloop3)
TEARDOWN_NOWRAP(nestedloop3)

SETUP(nestedloop2)
BODY(nestedloop2)
TEARDOWN_NOWRAP(nestedloop2)

SETUP(nestedloop)
BODY(nestedloop)
TEARDOWN_NOWRAP(nestedloop)

SETUP(random2)
BODY(random2)
TEARDOWN_NOWRAP(random2)

SETUP(random)
BODY(random)
TEARDOWN_NOWRAP(random)

SETUP(ratelimit)
BODY(ratelimit)
TEARDOWN(ratelimit)

SETUP(sieve)
BODY(sieve)
TEARDOWN_NOWRAP(sieve)

SETUP(ctype)
BODY(ctype)
TEARDOWN_NOWRAP(ctype)

SETUP_NOWRAP(switch)
BODY(switch)
TEARDOWN_NOWRAP(switch)

SETUP_NOWRAP(switch2)
BODY(switch2)
TEARDOWN_NOWRAP(switch2)

SETUP_NOWRAP(strcat2)
BODY(strcat2)
TEARDOWN_NOWRAP(strcat2)

SETUP_NOWRAP(strcat)
BODY(strcat)
TEARDOWN_NOWRAP(strcat)

SETUP(strchr)
BODY(strchr)
TEARDOWN_NOWRAP(strchr)

SETUP(strlen)
BODY(strlen)
TEARDOWN_NOWRAP(strlen)

SETUP(strtok)
BODY(strtok)
TEARDOWN_NOWRAP(strtok)

SETUP(memmove)
BODY(memmove)
TEARDOWN_NOWRAP(memmove)

SETUP_NOWRAP(xchacha20)
BODY(xchacha20)
TEARDOWN_NOWRAP(xchacha20)

SETUP_NOWRAP(xblabla20)
BODY(xblabla20)
TEARDOWN_NOWRAP(xblabla20)
