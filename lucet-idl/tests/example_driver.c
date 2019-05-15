#include <stdlib.h>
#include <assert.h>
#include "example.h"

int main (int argc, char *argv[]) {

    enum color c1 = COLOR_RED;
    colour c2 = COLOR_BLUE;
    col c3 = COLOR_GREEN;

    struct st s = {
        .a = 0,
        .b = 123,
        .c = COLOR_RED,
    };

    return 0;
}
