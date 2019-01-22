#include <string.h>

#include "inttypes.h"

#include "../include/lucet.h"
#include "../src/lucet_instance_private.h"
#include "../src/lucet_module_private.h"
#include "../src/lucet_sparse_page_data_private.h"
#include "greatest.h"
#include "guest_module.h"

#define VALID_SANDBOX_PATH "sparse_page_data/valid_sparse_page_data.so"

TEST test_valid_sparse_page_data(void)
{
    const char first_page[4096] = "hello from valid_sparse_page_data.c!";
    const char third_page[4096] = "hello again from valid_sparse_page_data.c!";

    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(VALID_SANDBOX_PATH));

    ASSERT(mod->sparse_page_data->num_pages == 3);
    ASSERT(memcmp(mod->sparse_page_data->pages[0], first_page, 4096) == 0);
    ASSERT(mod->sparse_page_data->pages[1] == NULL);
    ASSERT(memcmp(mod->sparse_page_data->pages[2], third_page, 4096) == 0);

    lucet_module_unload(mod);

    PASS();
}

TEST instantiate_valid_sparse_data(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(VALID_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // The test data initializers result in two strings getting copied into linear memory at the
    // beginnings of the first and third host pages
    const char *first_message = "hello from valid_sparse_page_data.c!";
    ASSERT(strcmp((char *) inst->alloc->heap, first_message) == 0);

    const char *second_message = "hello again from valid_sparse_page_data.c!";
    ASSERT(strcmp((char *) inst->alloc->heap + (4 * 1024 * 2), second_message) == 0);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

SUITE(sparse_page_data_suite)
{
    RUN_TEST(test_valid_sparse_page_data);
    RUN_TEST(instantiate_valid_sparse_data);
}
