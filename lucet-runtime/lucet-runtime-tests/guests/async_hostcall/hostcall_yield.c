#include <stddef.h>

extern void hostcall_containing_yield(int);

int main(void)
{
    hostcall_containing_yield(1312);
    return 0;
}
