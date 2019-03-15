#include <ctype.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>

// The WASI implementation of assert pulls facilities for in printing to stderr
// and aborting. This is lighter weight for a unit test
static void assert(bool v) {
    if (!v) {
        __builtin_unreachable();
    }
}

extern void black_box(void *);
#define BLACK_BOX(n) black_box((void *) &n)

#define STR_SIZE 50000
#define ITERATIONS 1000

typedef struct CtypeCtx_ {
    char * str;
    size_t str_size;
    size_t ret;
} CtypeCtx;

void ctype_setup(void *global_ctx, void **ctx_p)
{
    (void) global_ctx;

    static CtypeCtx ctx;
    ctx.str_size = STR_SIZE;

    *ctx_p = (void *) &ctx;
}

void ctype_body(void *ctx_)
{
    CtypeCtx *ctx = (CtypeCtx *) ctx_;

    char * str;
    size_t ret = (size_t) 0U;
    size_t j;
    int    i;

    ctx->str = malloc(ctx->str_size);
    assert(ctx->str != NULL);
    str = ctx->str;

    assert(ctx->str_size >= 2U);
    memset(ctx->str, 'x', ctx->str_size);
    ctx->str[ctx->str_size - 1U] = 0;

    for (i = 0; i < ITERATIONS; i++) {
        BLACK_BOX(ctx->str);
        for (j = 0U; j < ctx->str_size; j++) {
            ret += isalpha((int) (unsigned char) ctx->str[j]);
            ret += isalnum((int) (unsigned char) ctx->str[j]);
            ret += isdigit((int) (unsigned char) ctx->str[j]);
            ret += isspace((int) (unsigned char) ctx->str[j]);
            ret += isprint((int) (unsigned char) ctx->str[j]);
            if (isupper((int) (unsigned char) ctx->str[j])) {
                ctx->str[j] = (char) tolower((int) (unsigned char) ctx->str[j]);
            } else if (islower((int) (unsigned char) ctx->str[j])) {
                ctx->str[j] = (char) toupper((int) (unsigned char) ctx->str[j]);
            }
        }
    }

    free(ctx->str);
    BLACK_BOX(ret);
    ctx->ret = ret;
}
