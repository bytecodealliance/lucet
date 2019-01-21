#include "vm.h"

void guest_func_main(struct vmctx *);

int main()
{
    struct VM *vm = make_vm();
    guest_func_main(get_vmctx(vm));
    return 0;
}
