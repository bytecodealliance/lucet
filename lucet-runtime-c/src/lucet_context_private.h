#ifndef LUCET_CONTEXT_PRIVATE_H
#define LUCET_CONTEXT_PRIVATE_H

#include <immintrin.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>

#include "lucet_export.h"
#include "lucet_val.h"

// All of the callee-saved general purpose registers for a function call.
#pragma pack(push, 1)
struct lucet_context_gprs {
    uint64_t rbx;
    uint64_t rsp;
    uint64_t rbp;
    uint64_t rdi;
    uint64_t r12;
    uint64_t r13;
    uint64_t r14;
    uint64_t r15;
};
#pragma pack(pop)

// All of the callee-saved floating-point registers for a function call.
#pragma pack(push, 1)
struct lucet_context_fprs {
    __m128 xmm0;
    __m128 xmm1;
    __m128 xmm2;
    __m128 xmm3;
    __m128 xmm4;
    __m128 xmm5;
    __m128 xmm6;
    __m128 xmm7;
};
#pragma pack(pop)

// A context is a set of registers and a sigmask
#pragma pack(push, 1)
struct __attribute__((aligned(64))) lucet_context {
    // These must be laid out first in the struct, in that order, because the
    // assembly code has hard-coded their offsets.
    struct lucet_context_gprs gpr;
    struct lucet_context_fprs fpr;
    // These must be laid out right after the registers in the struct for similar reasons.
    uint64_t retvals_gp[2];
    __m128   retval_fp;
    // Used to restore the signal mask when jumping from a signal handler
    sigset_t sigset;
};
#pragma pack(pop)

_Static_assert(offsetof(struct lucet_context, gpr) == 0,
               "lucet_context.gpr is expected at offset 0");
_Static_assert(offsetof(struct lucet_context, fpr) == 8 * 8,
               "lucet_context.fpr is expected at offset 8*8");
_Static_assert(offsetof(struct lucet_context, retvals_gp) == 8 * 8 + 8 * 16,
               "lucet_context.retvals_gp is expected at offset 8*8 + 8*16");
_Static_assert(offsetof(struct lucet_context, retval_fp) == 8 * 8 + 8 * 16 + 8 * 2,
               "lucet_context.retval_fp is expected at offset 8*8 + 8*16 + 8*2");

// Create a context, given a stack, parent context, the function to run, and its
// arguments. Returns 0 on success, -1 if the arguments list is invalid.
int lucet_context_init(struct lucet_context *ctx, // A set of registers that defines the context
                       char *                stack_top, // top of stack (highest memory address)
                       struct lucet_context *parent,    // context to swap to if this one returns
                       void (*fptr)(),                  // Function that will be called at the start
                       void *vmctx_arg, // the vmctx is always the first argument to fptr.
                       int   argc,      // number of var-args, passed after vmctx_arg.
                       ...) // var-args. must all be ints (pointers into guest memory ok, no
                            // floats!)
    EXPORTED;

// Same as above except it takes a va_list.
int lucet_context_init_v(struct lucet_context *ctx, char *stack_top, struct lucet_context *parent,
                         void (*fptr)(), void *vmctx_arg, int argc, va_list argv) EXPORTED;

// Saves the caller's context in *from.
// Swap to the context provided in *to.
void lucet_context_swap(struct lucet_context *from, struct lucet_context const *to) EXPORTED;

// Swap to the context provided in *to without saving caller's context.
void lucet_context_set(struct lucet_context const *to) EXPORTED;

// Swap to the context provided in *to without saving caller's context.
// Also manages the return from a signal handler.
void lucet_context_set_from_signal(struct lucet_context const *to) EXPORTED;

// Clear (zero) return values.
void lucet_context_clear_retvals(struct lucet_context *ctx) EXPORTED;

// Get a return value from a context. Note that there is no validation that
// this was previously set by a `lucet_context_backstop`. If that is not the case
// the result will be undefined.
//
// Valid indexes are 0 and 1.
uint64_t lucet_context_get_retval_gp(struct lucet_context const *ctx, int idx) EXPORTED;

// Get the floating-point return value from a context.
__m128 lucet_context_get_retval_fp(struct lucet_context const *ctx) EXPORTED;

#endif // LUCET_CONTEXT_PRIVATE_H
