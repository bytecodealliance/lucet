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

int main()
{
    struct VM *vm    = make_vm();
    uint32_t   res_0 = guest_func_foo(get_vmctx(vm), 1);
    uint32_t   res_1 = guest_func_foo(get_vmctx(vm), 2);
    assert(res_0 == 1);
    assert(res_1 == 2);

    // Table elements 0, 4, and 5 are empty.
    assert(guest_table_0[0].type_tag == UINT64_MAX);
    assert(guest_table_0[0].function_ptr == NULL);
    assert(guest_table_0[4].type_tag == UINT64_MAX);
    assert(guest_table_0[4].function_ptr == NULL);
    assert(guest_table_0[5].type_tag == UINT64_MAX);
    assert(guest_table_0[5].function_ptr == NULL);

    // Table functions 1 and 2 have the same type. Function 3 has a different type.
    assert(guest_table_0[1].type_tag == guest_table_0[2].type_tag);
    assert(guest_table_0[1].type_tag != guest_table_0[3].type_tag);

    // Non-empty table functions point to a valid function
    assert(guest_table_0[1].function_ptr != NULL);
    assert(guest_table_0[2].function_ptr != NULL);
    assert(guest_table_0[3].function_ptr != NULL);

    return 0;
}
