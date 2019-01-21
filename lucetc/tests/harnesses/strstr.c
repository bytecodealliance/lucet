#include "vm.h"

void guest_func_strstr(struct vmctx *);

int main()
{
    struct VM *vm = make_vm();
    guest_func_strstr(get_vmctx(vm), 0, 0, 0);

    return 0;
}
