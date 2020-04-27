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
	push %ebp
	.cfi_def_cfa_offset 8
	.cfi_offset %ebp, -8
	mov	%esp, %ebp
	.cfi_def_cfa_register %ebp
	mov	%edi, -4(%ebp)
	#APP
	ud2
	#NO_APP
	pop	%ebp
	.cfi_def_cfa %esp, 4
	ret
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
	push	%ebp
	.cfi_def_cfa_offset 8
	.cfi_offset %ebp, -8
	mov	%esp, %ebp
	.cfi_def_cfa_register %ebp
	sub	$8, %esp
	mov	%edi, -4(%ebp)
	mov	-4(%ebp), %edi
#ifdef __ELF__
	call	lucet_vmctx_get_heap@PLT
#else
	call	_lucet_vmctx_get_heap
#endif
	mov	%eax, -8(%ebp)
	mov	-8(%ebp), %eax
	movb	$0, 65537(%eax)
	add	$8, %esp
	pop	%ebp
	.cfi_def_cfa %esp, 4
	ret
.Lfunc_end1:
#ifdef __ELF__
	.size   guest_func_oob, .Lfunc_end1-guest_func_oob
#endif
	.cfi_endproc

#if defined(__linux__) && defined(__ELF__)
	.section	".note.GNU-stack","",@progbits
#endif
