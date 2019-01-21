#include "vm.h"
#include <assert.h>
#include <stddef.h>
#include <stdint.h>

uint32_t guest_func_foo(struct vmctx *, uint32_t icall_dest);

struct table_elem {
    uint64_t type_tag;
    void *   function_ptr;
};
extern struct table_elem guest_table_0[];
extern uint8_t           guest_table_0_len[8];

int main()
{
    struct VM *vm    = make_vm();
    uint32_t   res_0 = guest_func_foo(get_vmctx(vm), 0);
    uint32_t   res_1 = guest_func_foo(get_vmctx(vm), 1);
    assert(res_0 == 1);
    assert(res_1 == 2);

    // Check table length
    assert(guest_table_0_len[0] == 3 * 2 * 8);
    assert(guest_table_0_len[1] == 0);
    assert(guest_table_0_len[2] == 0);
    assert(guest_table_0_len[3] == 0);
    assert(guest_table_0_len[4] == 0);
    assert(guest_table_0_len[5] == 0);
    assert(guest_table_0_len[6] == 0);
    assert(guest_table_0_len[7] == 0);

    // Table functions 0 and 1 have the same type. Function 2 has a different type.
    assert(guest_table_0[0].type_tag == guest_table_0[1].type_tag);
    assert(guest_table_0[0].type_tag != guest_table_0[2].type_tag);

    // All table functions point to a valid function
    assert(guest_table_0[0].function_ptr != NULL);
    assert(guest_table_0[1].function_ptr != NULL);
    assert(guest_table_0[2].function_ptr != NULL);

    return 0;
}
