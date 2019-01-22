#include "greatest.h"
#include "lucet_module.h"

SUITE_EXTERN(data_seg_init_suite);
SUITE_EXTERN(guest_fault_suite);
SUITE_EXTERN(features_suite);
SUITE_EXTERN(context_suite);
SUITE_EXTERN(entrypoint_suite);
SUITE_EXTERN(stats_suite);
SUITE_EXTERN(alloc_suite);
SUITE_EXTERN(alloc_context_suite);
SUITE_EXTERN(globals_suite);
SUITE_EXTERN(sparse_page_data_suite);

GREATEST_MAIN_DEFS();

int main(int argc, char **argv)
{
    GREATEST_MAIN_BEGIN(); /* command-line arguments, initialization. */

    lucet_report_load_errors(true);

    RUN_SUITE(data_seg_init_suite);
    RUN_SUITE(guest_fault_suite);
    RUN_SUITE(features_suite);
    RUN_SUITE(context_suite);
    RUN_SUITE(entrypoint_suite);
    RUN_SUITE(stats_suite);
    RUN_SUITE(alloc_suite);
    RUN_SUITE(alloc_context_suite);
    RUN_SUITE(globals_suite);
    RUN_SUITE(sparse_page_data_suite);

    GREATEST_MAIN_END(); /* display results */
}
