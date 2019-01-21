#ifndef __NR_restart_syscall
#include <bits/syscall.h>
#endif

#define a_cas a_cas
static inline int a_cas(volatile int *p, int t, int s)
{
	int old = *p;
	if (old == t)
		*p = s;
	return old;
}

#define a_crash() abort()
