#include "vm.h"
#include <assert.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

uint32_t guest_func_launchpad(struct vmctx *, uint32_t icall_dest, uint32_t input_a,
                              uint32_t input_b);

bool          expect_icall_to_env = false;
struct vmctx *expected_vmctx      = NULL;

uint32_t icalltarget(struct vmctx *ctx, uint32_t input_a, uint32_t input_b)
{
    assert(expect_icall_to_env);
    assert(ctx == expected_vmctx);
    return input_a * input_b;
}

int main()
{
    struct VM *vm = make_vm();

    // Table entry 0 adds the two inputs
    uint32_t res_0 = guest_func_launchpad(get_vmctx(vm), 0, 123, 456);
    assert(res_0 == (123 + 456));

    uint32_t res_1 = guest_func_launchpad(get_vmctx(vm), 0, 789, 10);
    assert(res_1 == (789 + 10));

    // Table entry 1 subtracts the two inputs
    uint32_t res_2 = guest_func_launchpad(get_vmctx(vm), 1, 123, 456);
    assert(res_2 == (123 - 456));

    uint32_t res_3 = guest_func_launchpad(get_vmctx(vm), 1, 789, 10);
    assert(res_3 == (789 - 10));

    // table entry #2 has wrong type.

    // Table entry 3 should call `icalltarget` above. That function multiplies
    // its two inputs.
    expect_icall_to_env = true;
    expected_vmctx      = get_vmctx(vm);
    uint32_t res_4      = guest_func_launchpad(get_vmctx(vm), 3, 123, 456);
    assert(res_4 == (123 * 456));

    return 0;
}
