#include "vm.h"

void guest_func_main(struct vmctx *);

int main()
{
    struct VM *vm = make_vm();
    guest_func_main(get_vmctx(vm));

    // fibonacci.wat writes the result of to mem location 0 as an i32.
    uint32_t output   = ((uint32_t *) vm->heap)[0];
    uint32_t expected = 21;
    if (output != expected) {
        printf("Output was %u\n", output);
        return 1;
    }
    return 0;
}
