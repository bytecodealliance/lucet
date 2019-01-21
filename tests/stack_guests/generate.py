import sys

"""
Code generator for wasm text files (.wat) that create a whole bunch of locals.
The code is designed to fool Cretonne into storing (almost) all of the locals
on the stack, by doing math with each local twice.

To control how much of the stack is used at run-time, the function takes a
parameter that determies its recursion depth. The function will call itself,
creating a new stack frame to store its locals, as many times as given by the
function's argument.

Note that, if just about anything in Cretonne gets smarter about stack
allocations or constant folding or subexpression elimination or reordering
recursions or who knows what else, the WebAssembly generated here could
probably be reduced down to some quick math that doesn't use the stack at all.

To determine how much stack space the native code generated from this webassembly
actually uses, you'll have to look at the disassembly (`objdump -d
build/session_guests/*.so`).

Tests in stack_suite should start to fail if something that is designed to
trigger a stack overflow starts terminating normally.

"""

start = '(module (func $localpalooza (export "localpalooza") (param i32) (result i32)'
end = '))'

def run(iters):
    assert(iters > 2)
    print(start)

    decl = '(local '
    for i in range(0,iters):
        decl += 'i32 '
    print(decl + ')')

    for i in range(0, iters):
        if i == 0:
            pass
        elif i == 1:
            print('(set_local 1 (i32.add (get_local 0) (i32.const 1)))')
        else:
            print('(set_local %d (i32.add (get_local %d) (get_local %d)))' % (i, i-1, i))

    # using every value twice gets them put on the stack between uses, which is
    # what we want - a big stack allocation
    for i in range(0, iters-1):
        if i > 1:
            print('(set_local %d (i32.add (get_local %d) (get_local %d)))' % (i, i-1, i))

    print('(if (get_local 0)')
    print('  (then (set_local %d (i32.add (get_local %d)' % (iters - 1, iters - 2))
    print('      (call $localpalooza (i32.sub (get_local 0) (i32.const 1))))))')
    print('  (else (set_local %d (i32.add (get_local %d) (get_local %d)))))' % (iters - 1, iters - 2, iters - 3))

    print('(get_local %d)' % (iters - 1))

    print(end)

if __name__ == "__main__":
    run(int(sys.argv[1]) + 1)
