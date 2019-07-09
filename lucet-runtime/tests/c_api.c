#include <stdbool.h>
#include <stdio.h>

#include "lucet.h"

bool lucet_runtime_test_expand_heap(struct lucet_dl_module *mod)
{
    struct lucet_region *     region;
    struct lucet_alloc_limits limits = {
        .heap_memory_size        = 4 * 1024 * 1024,
        .heap_address_space_size = 8 * 1024 * 1024,
        .stack_size              = 64 * 1024,
        .globals_size            = 4096,
    };

    enum lucet_error err;

    err = lucet_mmap_region_create(1, &limits, &region);
    if (err != lucet_error_ok) {
        fprintf(stderr, "failed to create region\n");
        goto fail1;
    }

    struct lucet_instance *inst;
    err = lucet_region_new_instance(region, mod, &inst);
    if (err != lucet_error_ok) {
        fprintf(stderr, "failed to create instance\n");
        goto fail2;
    }

    uint32_t newpage_start;
    err = lucet_instance_grow_heap(inst, 1, &newpage_start);
    if (err != lucet_error_ok) {
        fprintf(stderr, "failed to grow memory\n");
        goto fail3;
    }

    lucet_region_release(region);
    lucet_dl_module_release(mod);

    return true;

fail3:
    lucet_instance_release(inst);
fail2:
    lucet_region_release(region);
fail1:
    lucet_dl_module_release(mod);
    return false;
}
