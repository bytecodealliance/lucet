#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "lucet.h"

#define NOP_NO_HEAP_SANDBOX_PATH "build/guests/nop_no_heap.so"
#define NOP_HEAP_1K_SANDBOX_PATH "build/guests/nop_heap_1k.so"
#define NOP_HEAP_4K_SANDBOX_PATH "build/guests/nop_heap_4k.so"
#define NOP_HEAP_16K_SANDBOX_PATH "build/guests/nop_heap_16k.so"
#define NOP_HEAP_64K_SANDBOX_PATH "build/guests/nop_heap_64k.so"
#define NOP_HEAP_256K_SANDBOX_PATH "build/guests/nop_heap_256k.so"

#define NSEC_PER_SEC 1000000000L

struct results {
    int64_t instantiate_duration;
    int64_t run_duration;
    int64_t release_duration;
};

#define TIMESPEC_DIFF(start, finish) \
    ((finish.tv_sec - start.tv_sec) * NSEC_PER_SEC + (finish.tv_nsec - start.tv_nsec));

void bench_nop(const char *path, int n, struct results *results)
{
    struct timespec      start_time, instantiate_time, run_time, release_time;
    struct lucet_module *mod;
    mod = lucet_module_load(path);
    assert(mod != NULL);

    struct lucet_alloc_limits alloc_limits = (struct lucet_alloc_limits){
        .heap_memory_size        = 16 * 64 * 1024,
        .heap_address_space_size = 8 * 1024 * 1024,
        .stack_size              = 128 * 1024,
        .globals_size            = 128 * 1024,
    };

    struct lucet_pool *pool;
    pool = lucet_pool_create(n, &alloc_limits);

    struct lucet_instance **insts;
    insts = calloc(n, sizeof(struct lucet_instance *));

    clock_gettime(CLOCK_MONOTONIC, &start_time);

    for (int i = 0; i < n; i++) {
        insts[i] = lucet_instance_create(pool, mod, NULL);
        assert(insts[i] != NULL);
    }

    clock_gettime(CLOCK_MONOTONIC, &instantiate_time);

    for (int i = 0; i < n; i++) {
        lucet_instance_run(insts[i], "main", 0);
    }

    clock_gettime(CLOCK_MONOTONIC, &run_time);

    for (int i = 0; i < n; i++) {
        lucet_instance_release(insts[i]);
    }

    clock_gettime(CLOCK_MONOTONIC, &release_time);

    lucet_module_unload(mod);
    lucet_pool_decref(pool);

    results->instantiate_duration = TIMESPEC_DIFF(start_time, instantiate_time);
    results->run_duration         = TIMESPEC_DIFF(instantiate_time, run_time);
    results->release_duration     = TIMESPEC_DIFF(run_time, release_time);
}

void run_bench_nop(const char *path, int iterations, struct results *results)
{
    printf("== Running %s for %d iterations ==\n", path, iterations);
    bench_nop(path, iterations, results);
    printf("instantiate: %ldns\n", results->instantiate_duration / iterations);
    printf("run: %ldns\n", results->run_duration / iterations);
    printf("release: %ldns\n", results->release_duration / iterations);
    printf("\n");
}

int main()
{
    struct results results;

    run_bench_nop(NOP_NO_HEAP_SANDBOX_PATH, 250, &results);
    run_bench_nop(NOP_HEAP_1K_SANDBOX_PATH, 250, &results);
    run_bench_nop(NOP_HEAP_4K_SANDBOX_PATH, 250, &results);
    run_bench_nop(NOP_HEAP_16K_SANDBOX_PATH, 250, &results);
    run_bench_nop(NOP_HEAP_64K_SANDBOX_PATH, 250, &results);
    run_bench_nop(NOP_HEAP_256K_SANDBOX_PATH, 250, &results);

    return 0;
}
