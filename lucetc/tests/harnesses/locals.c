#include "vm.h"

uint32_t guest_func_main(struct vmctx *);

int main()
{
    struct VM *vm       = make_vm();
    uint32_t   ret      = guest_func_main(get_vmctx(vm));
    uint32_t   expected = 74;
    if (ret != expected) {
        printf("Output was %u, expected %u\n", ret, expected);
        return 1;
    }

    return 0;
}
