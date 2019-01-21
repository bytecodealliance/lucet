#include "vm.h"
#include <assert.h>
#include <stdbool.h>

void guest_func_main(struct vmctx *);

uint32_t arg      = 0;
bool     got_call = false;

uint32_t lucet_vmctx_grow_memory(struct VM *unused, uint32_t pages)
{
    got_call = true;
    arg      = pages;
    return 1234;
}

int main()
{
    struct VM *vm = make_vm();
    guest_func_main(get_vmctx(vm));

    uint32_t res = *(uint32_t *) vm->heap;
    assert(got_call);
    assert(res == 1234);
    assert(arg = 5678);

    return 0;
}
