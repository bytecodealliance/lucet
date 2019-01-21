#include <assert.h>
#include <err.h>
#include <immintrin.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#include "lucet_context_private.h"
#include "lucet_val_private.h"

// # Design notes
//
// This module was written as a replacement for our use of <ucontext.h>
// makecontext, setcontext, and swapcontext. It is only compatible with the
// x86_64 sysv abi.
//
// The goal of this code is to provide a separate stack (context) for the guest
// code to run in, so that the guest code may be paused and resumed without
// connection to the host thread's stack. At this time, the rest of liblucet does
// not implement the functionality required for pause/resume, but we wanted the
// execution model to support this so we don't have to make major changes later.
//
// ## Rationale
//
// We replaced the use of ucontext for two reasons:
//
// 1. We need to invoke the guest code with va_list of arguments created in the
// parent context. glibc makecontext supports variadic arguments but there is
// not a version of it that can take a va_list. We need a version that can take
// a va_list in order to expose a variadic lucet_run function.
//
// 2. Performance of ucontext is hampered by each set or swapcontext making a
// sigprocmask syscall. Our host and guest (sometimes called parent and child
// herein) contexts do not need to mask different signals. We only need to
// restore the signal mask when we are swapping contexts from within a signal
// handler. We did not quantify the performance hit of ucontext's syscalls but
// many other coroutine and greenthreads implementors, including our own
// @lbuytenhek, said they switched to a custom version of ucontext specifically
// to avoid this.
//
// ## Licensing notes
//
// The initial pass at this code was derived from
// https://github.com/Xudong-Huang/generator-rs/blob/master/src/detail/x86_64_unix.rs
// at commit 4f18fcb628595744ad188314ee8296f318830fde. The swap context routine
// is still the same but everything else has been rewritten. The MIT license for
// that code is provided in lucet_context_asm.S
//
// ## Context initialization
//
// When we initialize a context with `lucet_context_init` we are touching three
// data structures:
//
// * The context stack, passed in by a pointer `char* stack_top` to one element
// beyond the end of the stack (this is OK, see UB note below), is initialized
// as described in the stack layout section below. Pointers to both the stack's
// own context, and the parent context, are written to the stack.
//
// * The context itself, `struct lucet_context *ctx, has its `gpr` array
// initialized with the register values that will be set in the machine
// registers by a future call to `lucet_context_swap`. The %rsp and %rbp register
// values in the context will point to locations on the context stack.
//
// * The parent context, `struct lucet_context *parent`, has its `sigset` member
// initialized with the current value of the signal mask. A pointer to this
// context is saved for the child context to return to. The child context can
// call `lucet_context_swap` to change to any other context, but if at any point
// it returns, it will swap back to the context saved in *parent.
//
// The `void (*fptr)()` argument is the code that will be run when the context
// is first swapped to. This is a function pointer that takes "any args", but
// the actual args it is required to take is, first, a vmctx pointer `void
// *vmctx_arg`, and after that, the arguments in `va_list argv`, of which there
// must be the number given in `int argc`. There is no limit to argc except that
// it cannot be negative. The library user is responsible for making sure that
// the storage requirements of argv do not exceed the size of the stack (I
// highly doubt this would happen but I guess you never know).
//
// The vmctx pointer is passed separately from the va_list of args for two reasons:
// 1. When creating a guest context, each entry point is always required to take
// a vmctx pointer as its first argument
// 2. The va_list comes from the variadic arguments to lucet_run whereas the vmctx
// pointer is created within lucet_run. There's no way to cons the vmctx pointer
// onto the front of the va_list, because va_lists are a bad hack.
//
// ### Note about the parent context
//
// The `lucet_context_init` function is used to setup a context given an entirely
// new stack and entry point, but there is no analog to `getcontext` that makes
// a copy of the current context into an `struct lucet_context *`. The idea is
// that the "parent" context, which we assume is one created by the kernel as
// process thread, is saved into an lucet_context* by a call to `lucet_context_swap`
// as it is swapped away from. So, when a child context is initialized, it is
// passed the pointer that the parent context *will be* saved into, because it
// is not yet saved.
//
// Likewise, if you cange the signal mask between initializing a child context
// and swapping to it, the mask stored during initialization will be the one
// restored if `lucet_context_set_from_signal` is ever called.
//
// If either of these are an issue, we can change the design to make it more
// flexible, but this covered all of the current use cases.
//
// ### Register layout
//
// The `lucet_context.gpr` member is a struct of 8 registers. Each is represented
// as a u64. These 8 registers are all of the *callee saved* registers in the
// x86_64 sysv abi. The *caller* is responsible for saving and restoring any
// other registers it doesn't want to be clobbered by the call. This way, we
// make sure the C compiler saves all other required registers to the stack
// before its call to `lucet_context_swap`.
//
// %rsp and %rbp, the stack pointer and frame pointer, are set to point to the
// return address put on the top (lowest mem address) of the stack, and to a
// placeholder byte at the bottom (highest mem address) of the stack,
// respectively.
//
// ### Stack layout
//
// The user provides a pointer to the item one index past the bottom of the
// stack. Since x86 stacks grow down, this is the highest memory addrss of the
// stack, and the highest element it is valid to make a pointer to.
//
// The stack pointer given is rounded down to a 16-byte boundary, which is a
// requirement of the SysV ABI. I'm not sure if we have met all of the
// stack alignment obligations of the ABI.
//
// The stack frames are laid out, from top (lowest mem addrss) to bottom, always
// with 8-bytes per item:
// * First, the address of `lucet_context_bootstrap`. This is where we want the
// very first swap to return to. The bootstrap function sets up the registers
// for a call into the user's code, and then returns into it.
// * Second, the address provided as the entry point to the user's code,
// `void (*fptr)()`. This is what we want the bootstrap func to return to.
// * Third, the address of `lucet_context_backstop`. This is where we want the
// user's code to return to. The backstop function uses the frame pointer %rbp,
// which points to the very bottom of the stack, to get the pointer of the
// parent context to swap back to.
// * Fourth, any arguments to fptr that did not fit in the registers (beyond the
// first 6). Each of these arguments is treated as a u64! A u32 (whether numeric
// or a wasm pointer into linear memory) will work just fine, the abi treats
// them identically except that u32 uses a smaller load function.
// * Fifth, the arguments to lucet_context_backstop.
// * Sixth, the call stack is terminated. The frame pointer %rbp is pointed
// to the second-to-last location, which is zero, and the last location also
// contains a zero. This terminates a call stack according to the x64 ABI.
// Importantly, it stops libunwind from walking any further, which may be into
// unmapped memory.
//
// ### Bootstrapping
//
// Bootstrapping (by executing `lucet_context_bootstrap`) exists so that the
// call argument registers are populated correctly before the user's code is
// called.
//
// We store the call arguments for 5 registers (%rsi, %rdx, %rcx, %r8, and %r9)
// in the lucet_context.gpr fields for 5 of the callee-saved registers (%r12,
// %r13, %r14, %r15, and %rbx, respectively).
// The first swap context copies the values from the lucet_context struct into the
// callee-saved registers. The call argument in %rdi is put in place by the
// swap. The bootstrap code copies the values from the callee-saved registers to
// the remaining call argument registers. The values left over in the
// callee-saved registers doesnt matter, because the user's code will ignore
// them.
//
// Any additional call arguments, beyond the first 6, are put on the stack. The
// bootstrap code doesn't need to do anything with them.
//
// ## Context swapping
//
// `lucet_context_swap`
//
// Takes two context pointers: first, a mutable context `from` to save the
// caller's context into, and an immutable context `to` to swap into.
//
// `lucet_context_set`
//
// Takes just one context pointer: an immutable context `to` to swap into.
// The caller's context is not saved anywhere. When the caller's context is
// exiting, saving it is not required, so only use this then.
//
// `lucet_context_set_from_signal`
//
// A variant of `lucet_context_set` that is for use from a signal handler.  Takes
// just one context pointer: an immutable context `to` to swap into.  Before
// itself calling `lucet_context_set`, it restores the signal mask to the one that
// was saved in the `to` context. The `to` context must have at some point been
// a `parent` in a call to `lucet_context_init` so that the mask value saved there
// is valid.
//
// When exiting a signal handler by context swapping, instead of letting the
// signal handler return, the signal handler will not call the `sigreturn`
// syscall that the kernel pushed onto its control stack as part of the signal
// entry. The only thing `sigreturn` does that we care about is restore the
// signal mask. If you exit the signal handler without restoring the signal
// mask, subsequent signals wont get delivered and things will end badly for
// you.

// Defined in lucet_context_asm.S
extern void lucet_context_bootstrap(void);
extern void lucet_context_backstop(void);

static void lucet_context_bootstrap_gp_ix_arg(struct lucet_context *ctx, int ix, uint64_t arg)
{
    // Index is the abi argument number. The first 6 integer arguments can be
    // put into registers.
    switch (ix) {
    case 0: // rdi lives across bootstrap
        ctx->gpr.rdi = arg;
        break;
    case 1: // bootstraps into rsi
        ctx->gpr.r12 = arg;
        break;
    case 2: // bootstraps into rdx
        ctx->gpr.r13 = arg;
        break;
    case 3: // bootstraps into rcx
        ctx->gpr.r14 = arg;
        break;
    case 4: // bootstraps into r8
        ctx->gpr.r15 = arg;
        break;
    case 5: // bootstrap into r9
        ctx->gpr.rbx = arg;
        break;
    default:
        errx(1, "%s() unexpected register index", __FUNCTION__);
    }
}

static void lucet_context_bootstrap_fp_ix_arg(struct lucet_context *ctx, int ix, __m128 arg)
{
    // Index is the abi argument number. The first 8 floating-point arguments can
    // be put into registers.
    switch (ix) {
    case 0:
        ctx->fpr.xmm0 = arg;
        break;
    case 1:
        ctx->fpr.xmm1 = arg;
        break;
    case 2:
        ctx->fpr.xmm2 = arg;
        break;
    case 3:
        ctx->fpr.xmm3 = arg;
        break;
    case 4:
        ctx->fpr.xmm4 = arg;
        break;
    case 5:
        ctx->fpr.xmm5 = arg;
        break;
    case 6:
        ctx->fpr.xmm6 = arg;
        break;
    case 7:
        ctx->fpr.xmm7 = arg;
        break;
    default:
        errx(1, "%s() unexpected register index", __FUNCTION__);
    }
}

static void lucet_context_zero(struct lucet_context *ctx)
{
    memset(&ctx->gpr, 0, sizeof ctx->gpr);
    memset(&ctx->fpr, 0, sizeof ctx->fpr);
    lucet_context_clear_retvals(ctx);
}

int lucet_context_init(struct lucet_context *ctx, // A set of registers that defines the context
                       char *                stack_top, // top of stack (highest memory address)
                       struct lucet_context *parent,    // context to swap to if this one returns.
                                                        // assume its the context we're in now.
                       void (*fptr)(),                  // Function that will be called at the start
                       void *vmctx_arg, // the vmctx is always the first argument to fptr.
                       int   argc,      // number of var-args
                       ...)
{ // var-args. must all be lucet_val (pointers into guest memory ok)
    va_list argv;
    va_start(argv, argc);
    int ret = lucet_context_init_v(ctx, stack_top, parent, fptr, vmctx_arg, argc, argv);
    va_end(argv);

    return ret;
}

int lucet_context_init_v(struct lucet_context *ctx, // A set of registers that defines the context
                         char *                stack_top, // top of stack (highest memory address)
                         struct lucet_context *parent,    // context to swap to if this one returns
                         void (*fptr)(),    // Function that will be called at the start
                         void *  vmctx_arg, // the vmctx is always the first argument to fptr.
                         int     argc,      // number of var-args
                         va_list argv)
{ // var-args. must all be lucet_val (pointers into guest memory ok)
    lucet_context_zero(ctx);

    int onstack_args_count  = 0;
    int fp_args_count       = 0;
    int fp_args_register_ix = 0;
    int gp_args_count       = 0;
    int gp_args_register_ix = 0;
    int stack_args_ix       = 3;

    va_list argv_iter;
    va_copy(argv_iter, argv);

    // First arg is the vmctx
    lucet_context_bootstrap_gp_ix_arg(ctx, 0, (uint64_t) vmctx_arg);
    gp_args_count++;
    gp_args_register_ix++;

    // Next 5 general-purpose registers and 8 floating-point registers come from the var args
    for (int args_ix = 0; args_ix < argc; args_ix++) {
        struct lucet_val              val            = va_arg(argv_iter, struct lucet_val);
        enum lucet_val_register_class register_class = lucet_val_register_class(&val);

        if (register_class == lucet_val_register_class_gp) {
            gp_args_count++;
            if (gp_args_count > 6) {
                onstack_args_count++;
                continue;
            }
            uint64_t v64;
            if (lucet_val_transmute_to_u64(&v64, &val) != 0) {
                return -1;
            }
            lucet_context_bootstrap_gp_ix_arg(ctx, gp_args_register_ix, v64);
            gp_args_register_ix++;
        } else if (register_class == lucet_val_register_class_fp) {
            fp_args_count++;
            if (fp_args_count > 8) {
                onstack_args_count++;
                continue;
            }
            __m128 v128;
            if (lucet_val_transmute_to___m128(&v128, &val) != 0) {
                return -1;
            }
            lucet_context_bootstrap_fp_ix_arg(ctx, fp_args_register_ix, v128);
            fp_args_register_ix++;
        } else {
            return -1;
        }
    }

    // UB notes: stack_top is pointing to one element beyond the end of the
    // array allocated for the stack. It is defined behavior to make a pointer
    // to this element as long as it is never dereferenced.

    // Start from the top of the stack, aligned to 16.
    uint64_t *sp = (uint64_t *) ((uint64_t) stack_top & (~(uint64_t) 0x0F));

    int stack_start = 3                        // the bootstrap ret addr, then guest func ret addr,
                                               // then the backstop ret addr
                      + onstack_args_count     // then any args to guest func that dont fit
                                               // in registers
                      + onstack_args_count % 2 // padding to keep the stack 16-byte aligned
                                               // when we spill an odd number of arguments
                      + 4;                     // then the backstop args and terminator

    // If there are more additional args to the guest function than available
    // registers, they have to be pushed on the stack underneath the return
    // address.
    if (onstack_args_count > 0) {
        gp_args_count = 1;
        fp_args_count = 0;
        va_copy(argv_iter, argv);
        for (int args_ix = 0; args_ix < argc; args_ix++) {
            const struct lucet_val        val            = va_arg(argv_iter, struct lucet_val);
            enum lucet_val_register_class register_class = lucet_val_register_class(&val);

            if (register_class == lucet_val_register_class_gp) {
                gp_args_count++;
                if (gp_args_count <= 6) {
                    continue;
                }
            } else if (register_class == lucet_val_register_class_fp) {
                fp_args_count++;
                if (fp_args_count <= 8) {
                    continue;
                }
            } else {
                errx(1, "%s() unexpected register class", __FUNCTION__);
            }
            uint64_t v64;
            if (lucet_val_transmute_to_u64(&v64, &val) != 0) {
                return -1;
            }
            sp[stack_args_ix - stack_start] = v64;
            stack_args_ix++;
        }
    }

    // Prepare the stack for a swap context that lands in the bootstrap function
    // swap will ret into the bootstrap function
    sp[0 - stack_start] = (uint64_t) lucet_context_bootstrap;

    // The bootstrap function returns into the guest function, fptr
    sp[1 - stack_start] = (uint64_t) fptr;

    // the guest function returns into lucet_context_backstop.
    sp[2 - stack_start] = (uint64_t) lucet_context_backstop;

    // if fptr ever returns, it returns to the backstop func. backstop needs
    // two arguments in its frame - first the context we are switching *out of*
    // (which is also the one we are creating right now) and the ctx we switch
    // back into. Note *parent might not be a valid ctx now, but it should be
    // when this ctx is started.
    sp[-4] = (uint64_t) ctx;
    sp[-3] = (uint64_t) parent;
    // Terminate the call chain.
    sp[-2] = 0;
    sp[-1] = 0;

    // RSP and RBP still remain to be initialized.
    // Stack pointer: this has the return address of the first function to be
    // run on the swap.
    ctx->gpr.rsp = (uint64_t) &sp[-stack_start];
    // Frame pointer: this is only used by the backstop code. It uses it to
    // locate the ctx and parent arguments set above.
    ctx->gpr.rbp = (uint64_t) &sp[-2];

    // Read the sigprocmask to be restored if we ever need to jump out of
    // a signal handler. If this isnt possible, die.
    int res = sigprocmask(0, NULL, &parent->sigset);
    if (res != 0)
        err(1, "%s() sigprocmask could not be retrieved", __FUNCTION__);

    return 0;
}

void lucet_context_set_from_signal(struct lucet_context const *to)
{
    sigprocmask(SIG_SETMASK, &to->sigset, NULL);
    lucet_context_set(to);
}

void lucet_context_clear_retvals(struct lucet_context *ctx)
{
    ctx->retvals_gp[0] = 0;
    ctx->retvals_gp[1] = 0;
    ctx->retval_fp     = _mm_setzero_ps();
}

uint64_t lucet_context_get_retval_gp(struct lucet_context const *ctx, int idx)
{
    assert(idx >= 0 && idx < 2);
    return ctx->retvals_gp[idx];
}

__m128 lucet_context_get_retval_fp(struct lucet_context const *ctx)
{
    return ctx->retval_fp;
}
