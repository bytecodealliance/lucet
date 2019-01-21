#include <stdint.h>

void b(uint32_t arg)
{
    *((char *) -arg) = 0;
}
