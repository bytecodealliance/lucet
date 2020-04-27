	.text
	.globl	guest_func_illegal_instr # -- Begin function guest_func_illegal_instr
#ifdef __ELF__
	.type   guest_func_illegal_instr,@function
#else
	.globl	_guest_func_illegal_instr
#endif
	.p2align	4, 0x90
guest_func_illegal_instr:               # @guest_func_illegal_instr
_guest_func_illegal_instr:
	.cfi_startproc
# %bb.0:
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	movq	%rdi, -8(%rbp)
	#APP
	ud2
	#NO_APP
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end0:
#ifdef ___ELF__
	.size   guest_func_illegal_instr, .Lfunc_end0-guest_func_illegal_instr
#endif
	.cfi_endproc
                                        # -- End function
	.globl	guest_func_oob          # -- Begin function guest_func_oob
#ifdef __ELF__
	.type	guest_func_oob,@function
#else
	.globl	_guest_func_oob
#endif
	.p2align	4, 0x90
guest_func_oob:                         # @guest_func_oob
_guest_func_oob:
	.cfi_startproc
# %bb.0:
	pushq	%rbp
	.cfi_def_cfa_offset 16
	.cfi_offset %rbp, -16
	movq	%rsp, %rbp
	.cfi_def_cfa_register %rbp
	subq	$16, %rsp
	movq	%rdi, -8(%rbp)
	movq	-8(%rbp), %rdi
#ifdef __ELF__
	callq	lucet_vmctx_get_heap@PLT
#else
	callq	_lucet_vmctx_get_heap
#endif
	movq	%rax, -16(%rbp)
	movq	-16(%rbp), %rax
	movb	$0, 65537(%rax)
	addq	$16, %rsp
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end1:
#ifdef __ELF__
	.size   guest_func_oob, .Lfunc_end1-guest_func_oob
#endif
	.cfi_endproc

#if defined(__linux__) && defined(__ELF__)
	.section	".note.GNU-stack","",@progbits
#endif
