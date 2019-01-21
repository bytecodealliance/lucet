#include <stdlib.h>
#include <stdint.h>
#include "libc.h"

#include "lucet_libc.h"

static void dummy()
{
}

/* atexit.c and __stdio_exit.c override these. the latter is linked
 * as a consequence of linking either __toread.c or __towrite.c. */
weak_alias(dummy, __funcs_on_exit);
weak_alias(dummy, __stdio_exit);

_Noreturn void exit(int code)
{
	__funcs_on_exit();
	__stdio_exit();
	lucet_libc_syscall_exit(code);
}
