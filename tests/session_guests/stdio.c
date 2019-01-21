#include "session_hostcalls.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

__attribute__((visibility("default"))) int main(void)
{
    fputs("hello, stdout!\n", stdout);
    fputs("hello, stderr!\n", stderr);

    int  testdigits = 12345;
    char teststr[]  = "teststr";

    char buf[256];
    int  buf_len = snprintf(buf, 256, "snprintf can format digits: %d and strings: \"%s\"\n",
                           testdigits, teststr);
    assert(buf_len > 0);

    fputs(buf, stdout);

    // If printf worked, this would be equivelant to the above. Unfortunately,
    // it is not, because there is some bug in the stdio write subsystem that
    // printf hits, but fputs does not.
    /*
            printf("printf should print digits: %d and strings: \"%s\"\n",
                            testdigits, teststr);
    */

    exit(0);
    return 0;
}
