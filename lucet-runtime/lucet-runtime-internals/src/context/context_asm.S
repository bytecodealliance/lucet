/*
   The lucet_context_swap function is taken from Xudong Huang's
   generator-rs project. Its MIT license is provided below.

   Copyright (c) 2017 Xudong Huang

   Permission is hereby granted, free of charge, to any
   person obtaining a copy of this software and associated
   documentation files (the "Software"), to deal in the
   Software without restriction, including without
   limitation the rights to use, copy, modify, merge,
   publish, distribute, sublicense, and/or sell copies of
   the Software, and to permit persons to whom the Software
   is furnished to do so, subject to the following conditions:

   The above copyright notice and this permission notice
   shall be included in all copies or substantial portions
   of the Software.

   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
   ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
   TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
   PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
   SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
   CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
   OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
   IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
   DEALINGS IN THE SOFTWARE.

*/

.text
.globl lucet_context_bootstrap
#ifdef __ELF__
.type lucet_context_bootstrap,@function
#else
.globl _lucet_context_bootstrap
#endif
.align 16
lucet_context_bootstrap:
_lucet_context_bootstrap:
    // Move each of the argument values into the corresponding call
    // argument register.
    pop %r9
    pop %r8
    pop %rcx
    pop %rdx
    pop %rsi
    pop %rdi

    /* the next thing on the stack is the guest function - return to it */
    ret
#ifdef __ELF__
.size lucet_context_bootstrap,.-lucet_context_bootstrap
#endif

.text
.globl libunwind_backstop_shim
.align 16
// unsure if `simple` is necessary here. GAS docs suggest this may omit some initial CFI instructions, and i want this to be as barren as possible for now.
.cfi_startproc simple
    // .cfi_personality 0,win

    // TODO: generate these CFI instructions, perhaps using gimli

    // The idea with the call frame information here is to make any unwinder (libgcc, gdb,
    // libunwind, etc) think that the previous frame is the call to `Context::swap()` that initially swapped
    // us from host context into guest context. This means indicating that the values of
    // callee-saved registers may be found within the saved host context, and that the canonical
    // frame address is in the host stack.

    // The values we want to use are behind two dereferences (address of the parent context, parent
    // context), so the normal `.cfi_*` assembler directives are not sufficient to specify their
    // location. Instead, we have to use `.cfi_escape` so that we can write DWARF expressions that
    // locate the values or their addresses. See the DWARF spec for more info.

    // Start by providing the canonical frame address. Even though unwinding runtimes _should_ be
    // able to figure this out based on the saved rsp value, things go wrong if this is missing.

    .cfi_escape 0x0f, /* DW_CFA_def_cfa_expression */ \
                8, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x08, /* DW_OP_plus_uconst (add 8 to the base of context to point to saved rsp) */ \
                0x06, /* deref */ \
                0x23, 0x08 /* add 8 to pop the return address from the call to lucet_context_swap */

    // Now provide the saved rsp value. Note that unlike the other callee-saved register
    // expressions, this is a `val-expression` so that we can increment the final value to account for
    // the extra `lucet_context_swap` call frame we want to skip over

    .cfi_escape 0x16, 0x07, /* DW_CFA_val_expression(7=rsp) */ \
                8, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x08, /* DW_OP_plus_uconst (add 8 to the base of context to point to saved rsp) */ \
                0x06, /* deref */ \
                0x23, 0x08 /* add 8 to pop the return address from the call to lucet_context_swap */

    // The remaining expressions are all very similar; they just return the address that points to
    // the corresponding field on the `Context` struct

    .cfi_escape 0x10, 0x03, /* DW_CFA_expression(3=rbx) */ \
                3, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06 /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                /* rbx is at offset 0 */

    .cfi_escape 0x10, 0x06, /* DW_CFA_expression(6=rbp) */ \
                5, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x10 /* DW_OP_plus_uconst (add 16 to the base of context to point to saved rbp) */

    .cfi_escape 0x10, 0x05, /* DW_CFA_expression(5=rdi) */ \
                5, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x18 /* DW_OP_plus_uconst (add 24 to the base of context to point to saved rdi) */

    .cfi_escape 0x10, 0x0c, /* DW_CFA_expression(12=r12) */ \
                5, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x20 /* DW_OP_plus_uconst (add 32 to the base of context to point to saved r12) */

    .cfi_escape 0x10, 0x0d, /* DW_CFA_expression(13=r13) */ \
                5, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x28 /* DW_OP_plus_uconst (add 40 to the base of context to point to saved r13) */

    .cfi_escape 0x10, 0x0e, /* DW_CFA_expression(14=r14) */ \
                5, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x30 /* DW_OP_plus_uconst (add 48 to the base of context to point to saved r14) */

    .cfi_escape 0x10, 0x0f, /* DW_CFA_expression(15=r15) */ \
                5, /* uleb128 length of expression bytes */ \
                0x76, 0x18, /* DW_OP_breg5 (put rbp + 24 on the stack; should be a pointer to the parent context's address) */ \
                0x06, /* DW_OP_deref should then leave the parent context pointer on the stack */ \
                0x23, 0x38 /* DW_OP_plus_uconst (add 56 to the base of context to point to saved r15) */

    .cfi_offset 16, -8 /* return address value is stored at is CFA -8 */

    // this nop is load-bearing. We set up the backstop with a cfi_personality function so that stack unwinding through
    // guest code ends at a handler which can recover and switch back to the host.
    // However: libunwind doesn't treat the start of `lucet_context_backstop` as `lucet_context_backstop` but as the end
    // of `lucet_context_bootstrap`. so to get the unwinder to reach information about this function, start the unwind
    // region slightly before the real address we use, and place an address after the start, unambiguously referring to
    // the backstop function, on the stack for guests (and libunwind!) to eventually reach.
    nop
.globl lucet_context_backstop
#ifdef __ELF__
.type lucet_context_backstop,@function
#else
.globl _lucet_context_backstop
#endif
lucet_context_backstop:
_lucet_context_backstop:
    // Note that `rbp` here really has no relation to any stack!
    // Instead, it's a pointer to the guest context.
    mov (10*8 + 8*16 + 8*2 + 16)(%rbp), %rdi /* load the parent context to forward values in return value registers */
    mov %rax, (10*8 + 8*16 + 8*0)(%rbp) /* store return values before swapping back -- offset is offsetof(struct lucet_context, retvals) */
    mov %rdx, (10*8 + 8*16 + 8*1)(%rbp)
    movdqu %xmm0, (10*8 + 8*16 + 8*2)(%rbp) /* floating-point return value */

    // load `backstop_callback`, but skip calling it if it's null
    mov (10*8 + 8*16 + 8*2 + 16 + 8)(%rbp), %rsi
    test %rsi, %rsi
#ifdef __ELF__
    jz no_backstop_callback@PLT
#else
    jz no_backstop_callback
#endif

    // load `callback_data`, arg 1
    mov (10*8 + 8*16 + 8*2 + 16 + 8 + 8)(%rbp), %rdi
    // call `backstop_callback`
    call *%rsi

no_backstop_callback:
    mov %rbp, %rdi /* load the guest context to the "from" argument */
    mov (10*8 + 8*16 + 8*2 + 16)(%rbp), %rsi /* load the parent context to the "to" argument */

#ifdef __ELF__
    jmp lucet_context_swap@PLT
#else
    jmp lucet_context_swap
#endif
.cfi_endproc
#ifdef __ELF__
.size lucet_context_backstop,.-lucet_context_backstop
#endif

.text
.globl lucet_context_swap
#ifdef __ELF__
.type lucet_context_swap,@function
#else
.globl _lucet_context_swap
#endif
.align 16
lucet_context_swap:
_lucet_context_swap:
    // store everything in offsets from rdi (1st arg)
    mov %rbx, (0*8)(%rdi)
    mov %rsp, (1*8)(%rdi)
    mov %rbp, (2*8)(%rdi)
    mov %rdi, (3*8)(%rdi)
    mov %r12, (4*8)(%rdi)
    mov %r13, (5*8)(%rdi)
    mov %r14, (6*8)(%rdi)
    mov %r15, (7*8)(%rdi)
    mov %rsi, (8*8)(%rdi)

    movdqu %xmm0, (10*8 + 0*16)(%rdi)
    movdqu %xmm1, (10*8 + 1*16)(%rdi)
    movdqu %xmm2, (10*8 + 2*16)(%rdi)
    movdqu %xmm3, (10*8 + 3*16)(%rdi)
    movdqu %xmm4, (10*8 + 4*16)(%rdi)
    movdqu %xmm5, (10*8 + 5*16)(%rdi)
    movdqu %xmm6, (10*8 + 6*16)(%rdi)
    movdqu %xmm7, (10*8 + 7*16)(%rdi)

    // load everything from offsets from rsi (2nd arg)
    mov (0*8)(%rsi), %rbx
    mov (1*8)(%rsi), %rsp
    mov (2*8)(%rsi), %rbp
    mov (3*8)(%rsi), %rdi
    mov (4*8)(%rsi), %r12
    mov (5*8)(%rsi), %r13
    mov (6*8)(%rsi), %r14
    mov (7*8)(%rsi), %r15

    movdqu (10*8 + 0*16)(%rsi), %xmm0
    movdqu (10*8 + 1*16)(%rsi), %xmm1
    movdqu (10*8 + 2*16)(%rsi), %xmm2
    movdqu (10*8 + 3*16)(%rsi), %xmm3
    movdqu (10*8 + 4*16)(%rsi), %xmm4
    movdqu (10*8 + 5*16)(%rsi), %xmm5
    movdqu (10*8 + 6*16)(%rsi), %xmm6
    movdqu (10*8 + 7*16)(%rsi), %xmm7

    // restore rsi when we're done with the context pointer
    mov (8*8)(%rsi), %rsi

    ret
#ifdef __ELF__
.size lucet_context_swap,.-lucet_context_swap
#endif

.text
.globl lucet_context_set
#ifdef __ELF__
.type lucet_context_set,@function
#else
.globl _lucet_context_set
#endif
.align 16
lucet_context_set:
_lucet_context_set:
    // load everything from offsets from rdi (1st arg)
    mov (0*8)(%rdi), %rbx
    mov (1*8)(%rdi), %rsp
    mov (2*8)(%rdi), %rbp
    mov (4*8)(%rdi), %r12
    mov (5*8)(%rdi), %r13
    mov (6*8)(%rdi), %r14
    mov (7*8)(%rdi), %r15
    mov (8*8)(%rdi), %rsi

    movdqu (10*8 + 0*16)(%rdi), %xmm0
    movdqu (10*8 + 1*16)(%rdi), %xmm1
    movdqu (10*8 + 2*16)(%rdi), %xmm2
    movdqu (10*8 + 3*16)(%rdi), %xmm3
    movdqu (10*8 + 4*16)(%rdi), %xmm4
    movdqu (10*8 + 5*16)(%rdi), %xmm5
    movdqu (10*8 + 6*16)(%rdi), %xmm6
    movdqu (10*8 + 7*16)(%rdi), %xmm7

    // load rdi from itself last
    mov (3*8)(%rdi), %rdi
    ret
#ifdef __ELF__
.size lucet_context_set,.-lucet_context_set
#endif

.text
.globl lucet_context_activate
#ifdef __ELF__
.type lucet_context_activate,@function
#else
.globl _lucet_context_activate
#endif
.align 16
// `lucet_context_activate` is essentially a function with three arguments:
//   * rdi: the data for the entry callback.
//   * rsi: the address of the entry callback.
//   * rbx: the address of the guest code to execute.
//
// See `lucet_runtime_internals::context::lucet_context_activate` for more info.
//
// Note that `rbx` is used to store the address of the guest code because it is
// a callee-saved register in the System V calling convention. It is also a
// non-violatile register on Windows, which is a nice benefit.
lucet_context_activate:
_lucet_context_activate:
    // First, we call the entry callback whose address is stored in `rsi`,
    // passing along the value of `rdi` as the first argument.
    call *%rsi
    // Now, jump to the guest code at the address in `rbx`.
    jmp *%rbx
#ifdef __ELF__
.size lucet_context_activate,.-lucet_context_activate
#endif

/* Mark that we don't need executable stack. */
#if defined(__linux__) && defined(__ELF__)
.section .note.GNU-stack,"",%progbits
#endif
