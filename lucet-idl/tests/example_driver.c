#include <stdlib.h>
#include <assert.h>
#include "example.h"

int main (int argc, char *argv[]) {

    color c1 = COLOR_RED;
    colour c2 = COLOR_BLUE;
    col c3 = COLOR_GREEN;

    int32_t* b = malloc(sizeof(int32_t));
    struct st s = {
        .a = 0,
        .b = &b,
        .c = COLOR_RED,
        .self = &s,
    };

    return 0;
}
