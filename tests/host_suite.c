#include "inttypes.h"

#include "greatest.h"
#include "lucet.h"
#include "test_helpers.h"
// #include "lucet_backtrace.h"
#include "lucet_vmctx.h"

#define NULL_MOD_PATH "host_guests/null.so"
#define OOB_MOD_PATH "host_guests/oob.so"
#define HELLO_MOD_PATH "host_guests/hello.so"
#define HOSTCALL_ERROR_MOD_PATH "host_guests/hostcall_error.so"
#define FPE_MOD_PATH "host_guests/fpe.so"

void hostcall_test_func_hostcall_error(struct lucet_vmctx *ctx)
{
    lucet_vmctx_terminate(ctx, (void *) __FUNCTION__);
}

void hostcall_test_func_hello(struct lucet_vmctx *ctx,
                              guest_ptr_t         hello_ptr,
                              guest_size_t        hello_len)
{
    bool *confirmed_hello = (bool *) lucet_vmctx_get_delegate(ctx);

    const char *heap  = (const char *) lucet_vmctx_get_heap(ctx);
    const char *hello = heap + (uintptr_t) hello_ptr;
    if (!lucet_vmctx_check_heap(ctx, (void *) hello, hello_len)) {
        lucet_vmctx_terminate(ctx, NULL);
    }

    if (strstr(hello, "hello") == hello) {
        *confirmed_hello = true;
    }
}

SUITE(host_suite);

TEST test_load_module(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    lucet_dl_module_release(mod);

    PASS();
}

TEST test_load_nonexistent_module(void)
{
    struct lucet_dl_module *mod;
    enum lucet_error        err = lucet_dl_module_load("nonexistent_sandbox", &mod);

    ASSERT_ENUM_EQ(lucet_error_dl, err, lucet_error_name);

    PASS();
}

TEST test_instantiate(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST test_run_null(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

// /*
//  * Searches backtrace for a frame corresponding to sym_name in filepath.
//  * Used to smoke test generating backtraces for faulting guest programs.
//  * Does not support searching for NULL filepaths or symbols.
//  */
// bool find_frame(const struct lucet_backtrace *backtrace, const char *filepath, char *sym_name)
// {
//     ASSERT(filepath != NULL && sym_name != NULL); // YAGNI
//     for (int i = 0; i < backtrace->count; i++) {
//         if (!backtrace->frames[i].file_name || !backtrace->frames[i].sym_name) {
//             continue;
//         }
//         if (!strcmp(backtrace->frames[i].file_name, filepath) &&
//             !(strcmp(backtrace->frames[i].sym_name, sym_name))) {
//             return true;
//         }
//     }
//     return false;
// }

TEST test_run_oob(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    enum lucet_error err = lucet_instance_run(inst, "main", 0, (struct lucet_val[]){});
    ASSERT_ENUM_EQ(lucet_error_runtime_fault, err, lucet_error_name);

    // // As a smoke test, just verify the entry point symbol is in the backtrace
    // struct lucet_backtrace backtrace;
    // lucet_backtrace_create(&backtrace, &state->u.fault.context);
    // ASSERT(find_frame(&backtrace, guest_module_path(path), "guest_func_main"));

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST test_run_hello(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    bool confirm_hello = false;

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance_with_ctx(region, mod, &confirm_hello, &inst));

    ASSERT_OK(lucet_instance_run(inst, "main", 0, (struct lucet_val[]){}));

    ASSERT(confirm_hello);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST test_run_hostcall_error(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    const enum lucet_error err = lucet_instance_run(inst, "main", 0, (struct lucet_val[]){});
    ASSERT_ENUM_EQ(lucet_error_runtime_terminated, err, lucet_error_name);

    struct lucet_state state;
    ASSERT_OK(lucet_instance_state(inst, &state));

    ASSERT_STR_EQ(state.val.terminated, "hostcall_test_func_hostcall_error");

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

TEST test_run_fpe(const char *path)
{
    struct lucet_dl_module *mod;
    ASSERT_OK(lucet_dl_module_load(guest_module_path(path), &mod));

    struct lucet_mmap_region *region;
    ASSERT_OK(lucet_mmap_region_create(1, NULL, &region));

    struct lucet_instance *inst;
    ASSERT_OK(lucet_mmap_region_new_instance(region, mod, &inst));

    const enum lucet_error err =
        lucet_instance_run(inst, "trigger_div_error", 1, (struct lucet_val[]){ LUCET_VAL_U64(0) });
    ASSERT_ENUM_EQ(lucet_error_runtime_fault, err, lucet_error_name);

    lucet_instance_release(inst);
    lucet_dl_module_release(mod);
    lucet_mmap_region_release(region);

    PASS();
}

SUITE(host_suite)
{
    RUN_TEST1(test_load_module, NULL_MOD_PATH);
    RUN_TEST(test_load_nonexistent_module);
    RUN_TEST1(test_instantiate, NULL_MOD_PATH);
    RUN_TEST1(test_run_null, NULL_MOD_PATH);
    RUN_TEST1(test_run_oob, OOB_MOD_PATH);
    RUN_TEST1(test_run_hello, HELLO_MOD_PATH);
    RUN_TEST1(test_run_hostcall_error, HOSTCALL_ERROR_MOD_PATH);
    RUN_TEST1(test_run_fpe, FPE_MOD_PATH);
}
