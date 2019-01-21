
#include <stdio.h>

int __lockfile(FILE *f)
{
	(void) f;
	return 0;
}

void __unlockfile(FILE *f)
{
	(void) f;
}
