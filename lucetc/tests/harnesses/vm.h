
#ifndef VM_H
#define VM_H

#include <stdint.h>

#define GLOBALS_SIZE 128
#define HEAP_SIZE (64 * 1024)

struct VM {
    int64_t  globals[GLOBALS_SIZE];
    int64_t *global_ptr; // This should be initialized to point at &vm.globals
    char     heap[HEAP_SIZE];
};

struct VM *make_vm(void);

struct vmctx;

struct vmctx *get_vmctx(struct VM *);

struct VM *get_vm(struct vmctx *);

#endif // VM_H
