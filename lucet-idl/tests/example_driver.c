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

    char *mixedbag = malloc(BYTES_MIXEDBAG);

    store_mixedbag_a(mixedbag, c1);
    assert(is_mixedbag_a(mixedbag));

    store_mixedbag_b(mixedbag, 420.0);
    assert(is_mixedbag_b(mixedbag));

    store_mixedbag_c(mixedbag, &s);
    assert(is_mixedbag_c(mixedbag));

    set_mixedbag_d(mixedbag);
    assert(is_mixedbag_d(mixedbag));

    return 0;
}
