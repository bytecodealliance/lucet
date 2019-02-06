#include "greatest.h"
#include "guest_module.h"
#include "lucet.h"

#define CALCULATOR_SANDBOX_PATH "entrypoint_guests/calculator.so"
#define USE_ALLOCATOR_SANDBOX_PATH "entrypoint_guests/use_allocator.so"
#define CTYPE_SANDBOX_PATH "entrypoint_guests/ctype.so"
#define CALLBACK_SANDBOX_PATH "entrypoint_guests/callback.so"

TEST test_calc_add_2(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // Add the two arguments
    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_2", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);
    ASSERT_EQ(123 + 456, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_10(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // Add all 10 arguments. Why 10? Because its more than will fit in
    // registers to be passed to `guest_add_10` by liblucet, so it will
    // make sure that the calling convention of putting stuff on the
    // stack is working.
    // A better test might be to use an operation that doesn't commute,
    // so we can verify that the order is correct.
    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_10", 10, LUCET_VAL_U64(1), LUCET_VAL_U64(2), LUCET_VAL_U64(3),
                           LUCET_VAL_U64(4), LUCET_VAL_U64(5), LUCET_VAL_U64(6), LUCET_VAL_U64(7),
                           LUCET_VAL_U64(8), LUCET_VAL_U64(9), LUCET_VAL_U64(10));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_mul_2(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // Same sort of test as add_2, but with a different entrypoint.
    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "mul_2", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);
    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(123 * 456, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_add_then_mul(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // Both of these entrypoints have individual tests above, make sure
    // that they work when called in sequential runs on the same instance as
    // well. Neither should store state anywhere besides heap[0].
    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "add_2", 2, LUCET_VAL_U64(111), LUCET_VAL_U64(222));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(111 + 222, res);

    enum lucet_run_stat const stat2 =
        lucet_instance_run(inst, "mul_2", 2, LUCET_VAL_U64(333), LUCET_VAL_U64(444));
    ASSERT_ENUM_EQ(lucet_run_ok, stat2, lucet_run_stat_name);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res2 = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);

    ASSERT_EQ(333 * 444, res2);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST test_calc_invalid_entrypoint(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALCULATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "invalid", 2, LUCET_VAL_U64(123), LUCET_VAL_U64(456));
    ASSERT_ENUM_EQ(lucet_run_symbol_not_found, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

#define TEST_REGION_INIT_VAL 123
#define TEST_REGION_SIZE 4

TEST allocator_create_region(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(USE_ALLOCATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    char *heap = lucet_instance_get_heap(inst);

    // First, we need to get an unused location in linear memory for the pointer
    // that will be passed as an argument to create_and_memset.
    int32_t new_page = lucet_instance_grow_memory(inst, 1);
    ASSERT(new_page > 0);
    // Wasm location:
    uint32_t loc_outval = new_page * LUCET_WASM_PAGE_SIZE;
    // C pointer to value:
    uint32_t *ptr_outval = (uint32_t *) &heap[loc_outval];

    // This function will call `malloc` for the given size, then `memset` the
    // entire region to the init_as argument. The pointer to the allocated
    // region gets stored in loc_outval.
    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "create_and_memset", 3, LUCET_VAL_C_INT(TEST_REGION_INIT_VAL), /* int init_as */
        LUCET_VAL_USIZE(TEST_REGION_SIZE),                                   /* size_t size */
        LUCET_VAL_GUEST_PTR(loc_outval));                                    /* char** ptr_outval */

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // The location of the created region should be in a new page that the
    // allocator grabbed from the runtime. That page will be above the one
    // we got above.
    uint32_t loc_region_1 = *ptr_outval;
    ASSERT(loc_region_1 > loc_outval);

    // Each character in the newly created region will match the expected value.
    for (int i = 0; i < TEST_REGION_SIZE; i++) {
        ASSERT_EQ(TEST_REGION_INIT_VAL, heap[loc_region_1 + i]);
    }

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

TEST allocator_create_region_and_increment(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(USE_ALLOCATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    char *heap = lucet_instance_get_heap(inst);

    // First, we need to get an unused location in linear memory for the pointer
    // that will be passed as an argument to create_and_memset.
    int32_t new_page = lucet_instance_grow_memory(inst, 1);
    ASSERT(new_page > 0);
    uint32_t  loc_outval = new_page * LUCET_WASM_PAGE_SIZE;
    uint32_t *ptr_outval = (uint32_t *) &heap[loc_outval];

    // Create a region and initialize it, just like above.
    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "create_and_memset", 3, LUCET_VAL_C_INT(TEST_REGION_INIT_VAL), /* int init as */
        LUCET_VAL_USIZE(TEST_REGION_SIZE),                                   /* size_t size */
        LUCET_VAL_GUEST_PTR(loc_outval));                                    /* char** ptr_outval */

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint32_t loc_region_1 = *ptr_outval;
    ASSERT(loc_region_1 > loc_outval);

    // The region is initialized as expected.
    for (int i = 0; i < TEST_REGION_SIZE; i++) {
        ASSERT_EQ(TEST_REGION_INIT_VAL, heap[loc_region_1 + i]);
    }

    // Then increment the first location in the region.
    enum lucet_run_stat const stat2 =
        lucet_instance_run(inst, "increment_ptr", 1, LUCET_VAL_GUEST_PTR(loc_region_1));

    ASSERT_ENUM_EQ(lucet_run_ok, stat2, lucet_run_stat_name);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Just the first location in the region should be incremented.
    for (int i = 0; i < TEST_REGION_SIZE; i++) {
        if (i == 0) {
            ASSERT_EQ(TEST_REGION_INIT_VAL + 1, heap[loc_region_1 + i]);
        } else {
            ASSERT_EQ(TEST_REGION_INIT_VAL, heap[loc_region_1 + i]);
        }
    }

    // Increment the first location again.
    enum lucet_run_stat const stat3 =
        lucet_instance_run(inst, "increment_ptr", 1, LUCET_VAL_GUEST_PTR(loc_region_1));

    ASSERT_ENUM_EQ(lucet_run_ok, stat3, lucet_run_stat_name);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Just the first location in the region should be incremented twice.
    for (int i = 0; i < TEST_REGION_SIZE; i++) {
        if (i == 0) {
            ASSERT_EQ(TEST_REGION_INIT_VAL + 2, heap[loc_region_1 + i]);
        } else {
            ASSERT_EQ(TEST_REGION_INIT_VAL, heap[loc_region_1 + i]);
        }
    }

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

#define TEST_REGION2_INIT_VAL 99
#define TEST_REGION2_SIZE 420

TEST allocator_create_two_regions(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(USE_ALLOCATOR_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    char *heap = lucet_instance_get_heap(inst);

    // Same as above
    int32_t new_page = lucet_instance_grow_memory(inst, 1);
    ASSERT(new_page > 0);
    uint32_t  loc_outval = new_page * LUCET_WASM_PAGE_SIZE;
    uint32_t *ptr_outval = (uint32_t *) &heap[loc_outval];

    // Same as above
    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "create_and_memset", 3, LUCET_VAL_C_INT(TEST_REGION_INIT_VAL), /* int init as */
        LUCET_VAL_USIZE(TEST_REGION_SIZE),                                   /* size_t size */
        LUCET_VAL_GUEST_PTR(loc_outval));                                    /* char** ptr_outval */

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint32_t loc_region_1 = *ptr_outval;
    ASSERT(loc_region_1 > loc_outval);

    // Create a second region.
    enum lucet_run_stat const stat2 = lucet_instance_run(
        inst, "create_and_memset", 3, LUCET_VAL_C_INT(TEST_REGION2_INIT_VAL), /* int init as */
        LUCET_VAL_USIZE(TEST_REGION2_SIZE),                                   /* size_t size */
        LUCET_VAL_GUEST_PTR(loc_outval)); /* char** ptr_outval */

    ASSERT_ENUM_EQ(lucet_run_ok, stat2, lucet_run_stat_name);
    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // The allocator should pick a spot *after* the first region for the second
    // one. (It doesn't have to, but it will.) This shows that the allocators
    // metadata (free list) is preserved between the runs.
    uint32_t loc_region_2 = *ptr_outval;
    ASSERT(loc_region_2 > loc_outval);
    ASSERT(loc_region_2 >= (loc_region_1 + TEST_REGION_SIZE));

    // After this, the first region and second region should be initialized as
    // expected.
    for (int i = 0; i < TEST_REGION_SIZE; i++) {
        ASSERT_EQ(TEST_REGION_INIT_VAL, heap[loc_region_1 + i]);
    }
    for (int i = 0; i < TEST_REGION2_SIZE; i++) {
        ASSERT_EQ(TEST_REGION2_INIT_VAL, heap[loc_region_2 + i]);
    }

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

void black_box(void *vmctx, void *val)
{
    (void) vmctx;
    (void) val;
}

TEST test_ctype(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CTYPE_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    char *heap = lucet_instance_get_heap(inst);

    // First, we need to get an unused location in linear memory to store the
    // pointer to the "context" for the test case.
    int32_t new_page = lucet_instance_grow_memory(inst, 1);
    ASSERT(new_page > 0);
    // Wasm location:
    uint32_t loc_ctxstar = new_page * LUCET_WASM_PAGE_SIZE;

    // Run the setup routine
    enum lucet_run_stat const stat = lucet_instance_run(
        inst, "ctype_setup", 2, LUCET_VAL_C_PTR(NULL), /* void* global_ctx  -- not used */
        LUCET_VAL_GUEST_PTR(loc_ctxstar));             /* void** ctx_p */

    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    // Grab the value of the pointer that the setup routine wrote:
    uint32_t const ctxstar = *((uint32_t *) &heap[loc_ctxstar]);

    ASSERT(ctxstar > 0);

    // Run the body routine
    enum lucet_run_stat const stat2 =
        lucet_instance_run(inst, "ctype_body", 1, LUCET_VAL_GUEST_PTR(ctxstar)); /* void* ctx_ */

    ASSERT_ENUM_EQ(lucet_run_ok, stat2, lucet_run_stat_name);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

uint64_t callback_hostcall(void *vmctx, uint32_t cb_id, uint64_t x)
{
    void *func = lucet_vmctx_get_func_from_id(vmctx, 0, cb_id);
    return (*(uint64_t(*)(void *, uint64_t)) func)(vmctx, x) + 1;
}

TEST test_callback(void)
{
    struct lucet_module *mod;
    mod = lucet_module_load(guest_module_path(CALLBACK_SANDBOX_PATH));
    ASSERT(mod != NULL);

    struct lucet_pool *pool;
    pool = lucet_pool_create(1, NULL);

    struct lucet_instance *inst;
    inst = lucet_instance_create(pool, mod, NULL);
    ASSERT(inst != NULL);

    // Add the two arguments
    enum lucet_run_stat const stat =
        lucet_instance_run(inst, "callback_entrypoint", 1, LUCET_VAL_U64(0));
    ASSERT_ENUM_EQ(lucet_run_ok, stat, lucet_run_stat_name);

    const struct lucet_state *state;
    state = lucet_instance_get_state(inst);

    ASSERT_ENUM_EQ(lucet_state_ready, state->tag, lucet_state_name);

    uint64_t res = LUCET_UNTYPED_RETVAL_TO_U64(state->u.ready.untyped_retval);
    ASSERT_EQ(3, res);

    lucet_instance_release(inst);
    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    PASS();
}

SUITE(entrypoint_suite)
{
    RUN_TEST(test_calc_add_2);
    RUN_TEST(test_calc_add_10);
    RUN_TEST(test_calc_mul_2);
    RUN_TEST(test_calc_add_then_mul);
    RUN_TEST(test_calc_invalid_entrypoint);
    RUN_TEST(allocator_create_region);
    RUN_TEST(allocator_create_region_and_increment);
    RUN_TEST(allocator_create_two_regions);
    RUN_TEST(test_ctype);
    RUN_TEST(test_callback);
}
