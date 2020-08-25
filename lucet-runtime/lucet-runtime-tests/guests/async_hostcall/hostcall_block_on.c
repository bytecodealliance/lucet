#include <stddef.h>

extern void hostcall_containing_block_on(int);

int main(void)
{
    hostcall_containing_block_on(1312);
    return 0;
}
