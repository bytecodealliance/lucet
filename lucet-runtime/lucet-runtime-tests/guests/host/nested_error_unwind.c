#include <stddef.h>

extern void nested_error_unwind_outer(void (*)(void));
extern void nested_error_unwind_inner();

void callback(void) {
    nested_error_unwind_inner();
}

void entrypoint(void)
{
    nested_error_unwind_outer(callback);
}
