#include <stdlib.h>
#include "lucet_libc.h"

_Noreturn void compilerrt_abort_impl(void) {
	lucet_libc_syscall_abort();
}
