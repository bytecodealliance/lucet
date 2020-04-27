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
//    pop %r9
//    pop %r8
    pop %ecx
    pop %edx
    pop %esi
    pop %edi

    /* the next thing on the stack is the guest function - return to it */
    ret
#ifdef __ELF__
.size lucet_context_bootstrap,.-lucet_context_bootstrap
#endif

.text
.globl lucet_context_backstop
#ifdef __ELF__
.type lucet_context_backstop,@function
#else
.globl _lucet_context_backstop
#endif
.align 16
lucet_context_backstop:
_lucet_context_backstop:
    // Note that `rbp` here really has no relation to any stack!
    // Instead, it's a pointer to the guest context.
    mov (10*8 + 8*16 + 8*2 + 16)(%ebp), %edi /* load the parent context to forward values in return value registers */
    mov %eax, (10*8 + 8*16 + 8*0)(%ebp) /* store return values before swapping back -- offset is offsetof(struct lucet_context, retvals) */
    mov %edx, (10*8 + 8*16 + 8*1)(%ebp)
    movdqu %xmm0, (10*8 + 8*16 + 8*2)(%ebp) /* floating-point return value */

    // load `backstop_callback`, but skip calling it if it's null
    mov (10*8 + 8*16 + 8*2 + 16 + 8)(%ebp), %esi
    test %esi, %esi
#ifdef __ELF__
    jz no_backstop_callback@PLT
#else
    jz no_backstop_callback
#endif

    // load `callback_data`, arg 1
    mov (10*8 + 8*16 + 8*2 + 16 + 8 + 8)(%ebp), %edi
    // call `backstop_callback`
    call *%esi

no_backstop_callback:
    mov %ebp, %edi /* load the guest context to the "from" argument */
    mov (10*8 + 8*16 + 8*2 + 16)(%ebp), %esi /* load the parent context to the "to" argument */

#ifdef __ELF__
    jmp lucet_context_swap@PLT
#else
    jmp lucet_context_swap
#endif
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
    mov %ebx, (0*8)(%edi)
    mov %esp, (1*8)(%edi)
    mov %ebp, (2*8)(%edi)
    mov %edi, (3*8)(%edi)
//    mov %r12, (4*8)(%rdi)
//    mov %r13, (5*8)(%rdi)
//    mov %r14, (6*8)(%rdi)
//    mov %r15, (7*8)(%rdi)
    mov %esi, (8*8)(%edi)

    movdqu %xmm0, (10*8 + 0*16)(%edi)
    movdqu %xmm1, (10*8 + 1*16)(%edi)
    movdqu %xmm2, (10*8 + 2*16)(%edi)
    movdqu %xmm3, (10*8 + 3*16)(%edi)
    movdqu %xmm4, (10*8 + 4*16)(%edi)
    movdqu %xmm5, (10*8 + 5*16)(%edi)
    movdqu %xmm6, (10*8 + 6*16)(%edi)
    movdqu %xmm7, (10*8 + 7*16)(%edi)

    // load everything from offsets from rsi (2nd arg)
    mov (0*8)(%esi), %ebx
    mov (1*8)(%esi), %esp
    mov (2*8)(%esi), %ebp
    mov (3*8)(%esi), %edi
//    mov (4*8)(%rsi), %r12
//    mov (5*8)(%rsi), %r13
//    mov (6*8)(%rsi), %r14
//    mov (7*8)(%rsi), %r15

    movdqu (10*8 + 0*16)(%esi), %xmm0
    movdqu (10*8 + 1*16)(%esi), %xmm1
    movdqu (10*8 + 2*16)(%esi), %xmm2
    movdqu (10*8 + 3*16)(%esi), %xmm3
    movdqu (10*8 + 4*16)(%esi), %xmm4
    movdqu (10*8 + 5*16)(%esi), %xmm5
    movdqu (10*8 + 6*16)(%esi), %xmm6
    movdqu (10*8 + 7*16)(%esi), %xmm7

    // restore rsi when we're done with the context pointer
    mov (8*8)(%esi), %esi

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
    mov (0*8)(%edi), %ebx
    mov (1*8)(%edi), %esp
    mov (2*8)(%edi), %ebp
//    mov (4*8)(%edi), %r12
//    mov (5*8)(%edi), %r13
//    mov (6*8)(%edi), %r14
//    mov (7*8)(%edi), %r15
    mov (8*8)(%edi), %esi

    movdqu (10*8 + 0*16)(%edi), %xmm0
    movdqu (10*8 + 1*16)(%edi), %xmm1
    movdqu (10*8 + 2*16)(%edi), %xmm2
    movdqu (10*8 + 3*16)(%edi), %xmm3
    movdqu (10*8 + 4*16)(%edi), %xmm4
    movdqu (10*8 + 5*16)(%edi), %xmm5
    movdqu (10*8 + 6*16)(%edi), %xmm6
    movdqu (10*8 + 7*16)(%edi), %xmm7

    // load rdi from itself last
    mov (3*8)(%edi), %edi
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
    call *%esi
    // Now, jump to the guest code at the address in `rbx`.
    jmp *%ebx
#ifdef __ELF__
.size lucet_context_activate,.-lucet_context_activate
#endif

/* Mark that we don't need executable stack. */
#if defined(__linux__) && defined(__ELF__)
.section .note.GNU-stack,"",%progbits
#endif
