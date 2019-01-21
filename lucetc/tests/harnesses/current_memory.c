#include "vm.h"
#include <assert.h>
#include <stdbool.h>

void guest_func_main(struct vmctx *);

bool got_call = false;

uint32_t lucet_vmctx_current_memory(struct vmctx *unused)
{
    got_call = true;
    return 1234;
}

int main()
{
    struct VM *vm = make_vm();
    guest_func_main(get_vmctx(vm));

    uint32_t res = *(uint32_t *) vm->heap;
    assert(got_call);
    assert(res == 1234);

    return 0;
}
