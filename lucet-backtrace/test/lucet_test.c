#include "greatest.h"

SUITE_EXTERN(backtrace_suite);

GREATEST_MAIN_DEFS();

int main(int argc, char **argv)
{
    GREATEST_MAIN_BEGIN(); /* command-line arguments, initialization. */

    RUN_SUITE(backtrace_suite);

    GREATEST_MAIN_END(); /* display results */
}
