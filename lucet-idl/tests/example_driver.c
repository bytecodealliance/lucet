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

    int32_t ctr_res = example_set_counter(666);

    enum color c4 = example_set_color(c1);

    example_set_struct(&s);

    example_get_color_to_ptr(&c1);

    example_swap_color_by_ptr(&c2);

    const uint8_t debug_str[] = "hello, world!";
    example_debug_str(debug_str, sizeof(debug_str));

    uint8_t inout_str[] = "hello again, I guess";
    example_inout_str(inout_str, sizeof(inout_str));

    return 0;
}
