
#include "globals.h"
#include "vm.h"
#include <assert.h>
#include <stdbool.h>

extern const struct global_table lucet_globals_spec;

void guest_func_main(struct vmctx *);

int main()
{
    struct VM *vm = make_vm();

    assert(vm->globals[0] == 0);
    assert(vm->globals[1] == 0);
    assert(vm->globals[2] == 0);

    initialize_globals(vm, &lucet_globals_spec);

    assert(vm->globals[0] == 4);
    assert(vm->globals[1] == 5);
    assert(vm->globals[2] == 6);

    guest_func_main(get_vmctx(vm));

    assert(vm->globals[0] == 3);
    assert(vm->globals[1] == 2);

    uint32_t *heap_globals = (uint32_t *) vm->heap;
    assert(heap_globals[0] == 4);
    assert(heap_globals[1] == 5);
    assert(heap_globals[2] == 6);

    return 0;
}
