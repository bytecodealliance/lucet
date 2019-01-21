#include "greatest.h"

SUITE_EXTERN(host_suite);
SUITE_EXTERN(session_suite);
SUITE_EXTERN(entrypoint_suite);
SUITE_EXTERN(memory_suite);
SUITE_EXTERN(strcmp_suite);
SUITE_EXTERN(stack_suite);
SUITE_EXTERN(globals_suite);
SUITE_EXTERN(start_suite);

GREATEST_MAIN_DEFS();

int main(int argc, char **argv)
{
    GREATEST_MAIN_BEGIN(); /* command-line arguments, initialization. */

    RUN_SUITE(host_suite);
    RUN_SUITE(session_suite);
    RUN_SUITE(entrypoint_suite);
    RUN_SUITE(memory_suite);
    RUN_SUITE(strcmp_suite);
    RUN_SUITE(stack_suite);
    RUN_SUITE(globals_suite);
    RUN_SUITE(start_suite);

    GREATEST_MAIN_END(); /* display results */
}
