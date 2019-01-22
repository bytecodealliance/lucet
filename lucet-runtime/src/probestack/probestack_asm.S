// Derived from Rustc's __rust_probestack implementation
// https://github.com/rust-lang-nursery/compiler-builtins/blob/master/src/probestack.rs
// Copyright 2017 The Rust Project Developers. Licensed under the Apache
// License, Version 2.0, or the MIT license, at your option.

// IMPORTANT:
// lucet_probestack_private.h defines a macro LUCET_PROBESTACK_SIZE that gives the
// size of the program text for the following function.  If you make any
// modifications to the below code, use `objdump -d build/lucet_probestack.o` to
// determine the code size, and update the constant.

.text
.globl lucet_probestack
.type lucet_probestack,@function
.align 16
lucet_probestack:
	// Our goal is to touch every page between %rsp+8 and %rsp+8-%rax,
	// ensuring that if any pages in that range are unmapped, we will
	// cause a page fault, which liblucet will recognize as a stack overflow.

	// The ABI for this function is special. The stack frame size is located
	// in %eax. %esp and %eax are to be preserved on return.

	mov %rax,%r11 // Duplicate %rax. We can clobber %r11.

	// Loop in one page increments. Decrement rsp by a page until less than a
	// page remaining. This function will only be called if more than one page is
	// needed.
	// Note that we are testing against 8(%rsp) to account for the 8 bytes pushed
	// on the stack originally with our return address. Using 8(%rsp) simulates us
	// the stack pointer in the caller's context.
	// A page is 0x1000 in size (4k).
	2:
	sub $0x1000,%rsp
	test %rsp,8(%rsp)
	sub $0x1000,%r11
	cmp $0x1000,%r11
	ja 2b

	// Finish the last remaining stack space requested, getting the last
	// bits out of r11
	sub %r11,%rsp
	test %rsp,8(%rsp)

	// Restore stack pointer to value it was upon entry
	add %rax,%rsp

	ret
.size lucet_probestack,.-lucet_probestack
.globl lucet_probestack_size
.type lucet_probestack_size,@object
lucet_probestack_size: .quad .-lucet_probestack

/* Mark that we don't need executable stack. */
.section .note.GNU-stack,"",%progbits
