#include <stddef.h>

extern void hostcall_containing_block_on(int);
extern void hostcall_containing_yielding_block_on(int);


int main(void)
{
    hostcall_containing_block_on(1312);
    return 0;
}

int yielding()
{
    hostcall_containing_yielding_block_on(0);
    hostcall_containing_yielding_block_on(1);
    hostcall_containing_yielding_block_on(2);
    hostcall_containing_yielding_block_on(3);
    return 0;
}