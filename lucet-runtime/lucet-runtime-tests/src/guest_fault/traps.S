	.text
	.globl	guest_func_illegal_instr # -- Begin function guest_func_illegal_instr
	.p2align	4, 0x90
	.type	guest_func_illegal_instr,@function
guest_func_illegal_instr:               # @guest_func_illegal_instr
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
	.size	guest_func_illegal_instr, .Lfunc_end0-guest_func_illegal_instr
	.cfi_endproc
                                        # -- End function
	.globl	guest_func_oob          # -- Begin function guest_func_oob
	.p2align	4, 0x90
	.type	guest_func_oob,@function
guest_func_oob:                         # @guest_func_oob
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
	callq	lucet_vmctx_get_heap@PLT
	movq	%rax, -16(%rbp)
	movq	-16(%rbp), %rax
	movb	$0, 65537(%rax)
	addq	$16, %rsp
	popq	%rbp
	.cfi_def_cfa %rsp, 8
	retq
.Lfunc_end1:
	.size	guest_func_oob, .Lfunc_end1-guest_func_oob
	.cfi_endproc
                                        # -- End function
	.ident	"clang version 7.0.1-svn348686-1~exp1~20181221231927.53 (branches/release_70)"
	.section	".note.GNU-stack","",@progbits
