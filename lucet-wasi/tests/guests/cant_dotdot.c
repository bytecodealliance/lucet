#include <assert.h>
#include <errno.h>
#include <stdio.h>

int main()
{
    FILE *file = fopen("/sandbox/../outside.txt", "r");
    assert(file == NULL);
    assert(errno == EPERM);

    return 0;
}
