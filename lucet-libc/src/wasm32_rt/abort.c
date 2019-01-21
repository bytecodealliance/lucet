#include <stdlib.h>
#include "lucet_libc.h"

_Noreturn void abort(void) {
	lucet_libc_syscall_abort();
}
