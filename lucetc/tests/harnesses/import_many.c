#include "vm.h"
#include <assert.h>

void guest_func_main(struct vmctx *);

uint32_t stage;

uint32_t imp_0(void)
{
    assert(stage == 0);
    stage = 1;
    return 1;
}

uint32_t imp_1(void)
{
    assert(stage == 1);
    stage = 2;
    return 2;
}

uint32_t imp_2(void)
{
    assert(stage == 2);
    stage = 3;
    return 3;
}

uint32_t imp_3(void)
{
    assert(stage == 3);
    stage = 4;
    return 4;
}

int main()
{
    struct VM *vm = make_vm();
    stage         = 0;
    guest_func_main(get_vmctx(vm));
    assert(stage == 4);

    return 0;
}
