#include <assert.h>
#include <dlfcn.h>
#include <err.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "lucet_module.h"
#include "lucet_module_private.h"
#include "lucet_stats_private.h"

#include "lucet_probestack_private.h"
#include "lucet_vmctx.h"

// This array exists to ensure that, if modules are ever loaded, the linker
// pulls in the lucet_vmctx functions they need to link with.
static void *lucet_vmctx_funcs[] = {
    (void *) lucet_vmctx_current_memory,
    (void *) lucet_vmctx_grow_memory,
    (void *) lucet_probestack,
};

static __thread bool lucet_rep_err = false;

bool lucet_report_load_errors(bool flag)
{
    bool o_flag   = lucet_rep_err;
    lucet_rep_err = flag;
    return o_flag;
}

struct lucet_module *lucet_module_load(const char *dl_path)
{
    // This statement is to ensure linker includes vmctx funcs:
    (void) lucet_vmctx_funcs;

    char *err;

    struct lucet_module *m = malloc(sizeof(struct lucet_module));
    assert(m);

    // Load the dynamic library. The undefined symbols corresponding to the
    // lucet_syscall_ functions will be provided by the current executable.
    // We trust our wasm->dylib compiler to make sure these function calls
    // are the way the dylib can touch memory outside of its stack and
    // heap.
    m->dl_handle = dlopen(dl_path, RTLD_NOW);
    // no error handling here, for the sake of the prototype.
    if (!m->dl_handle) {
        if (lucet_rep_err) {
            fprintf(stderr, "dlopen error: %s\n", dlerror());
        }
        goto error;
    }

    // Load the module tables (there is only one for now)
    struct lucet_table_element *p_table_segment = dlsym(m->dl_handle, "guest_table_0");
    uint64_t *p_table_segment_len = (uint64_t *) dlsym(m->dl_handle, "guest_table_0_len");
    m->table_elements = (struct lucet_table_elements){ .elements = NULL, .last_id = 0U };
    if (p_table_segment != NULL && p_table_segment_len != NULL) {
        if (*p_table_segment_len >
                (uint64_t) UINT32_MAX * (uint64_t) sizeof(struct lucet_table_element) ||
            *p_table_segment_len % (uint64_t) sizeof(struct lucet_table_element) != 0U) {
            fprintf(stderr, "unexpected table segment length: %llu\n",
                    (unsigned long long) *p_table_segment_len);
            goto error;
        }
        if (p_table_segment_len > 0U) {
            m->table_elements = (struct lucet_table_elements){
                .elements = p_table_segment,
                .last_id  = (uint32_t)(
                    *p_table_segment_len / (uint64_t) sizeof(struct lucet_table_element) - 1U)
            };
        }
    }

    // Load WASM data segment initialization info from .so if it is present.
    // This will be used to initialize linear memory when the module is
    // instantiated.
    // Logical validation happens after heap spec loaded.
    void *p_data_segments = dlsym(m->dl_handle, "wasm_data_segments");
    err                   = dlerror();
    if (err) {
        if (lucet_rep_err) {
            fprintf(stderr, "missing data segments. dlsym_error: %s\n", err);
            goto error;
        }
    }
    if (p_data_segments == NULL) {
        if (lucet_rep_err) {
            fprintf(stderr, "null data segments\n");
        }
        goto error;
    }

    uint32_t *p_seg_len = (uint32_t *) dlsym(m->dl_handle, "wasm_data_segments_len");
    err                 = dlerror();
    if (err) {
        if (lucet_rep_err) {
            fprintf(stderr, "missing data segments length. dlsym_error: %s\n", err);
        }
        goto error;
    }
    if (p_seg_len == NULL) {
        if (lucet_rep_err) {
            fprintf(stderr, "null data segments length\n");
        }
        goto error;
    }

    m->data_segment = (struct lucet_data_segment_descriptor){
        .segments = p_data_segments,
        .len      = *p_seg_len,
    };

    // Optional: if missing, ignore.
    m->sparse_page_data = dlsym(m->dl_handle, "guest_sparse_page_data");

    struct lucet_alloc_heap_spec *heap_spec = dlsym(m->dl_handle, "lucet_heap_spec");
    err                                     = dlerror();
    if (err) {
        if (lucet_rep_err) {
            fprintf(stderr, "missing wasm memory spec. dlsym_error: %s\n", err);
        }
        goto error;
    }
    if (heap_spec == NULL) {
        if (lucet_rep_err) {
            fprintf(stderr, "null wasm memory spec\n");
        }
        goto error;
    }

    bool data_segment_valid = lucet_data_segment_validate(&m->data_segment, heap_spec);
    if (!data_segment_valid) {
        if (lucet_rep_err) {
            fprintf(stderr, "data segment is invalid according to heap spec\n");
        }
        goto error;
    }

    struct lucet_globals_spec *globals_spec = dlsym(m->dl_handle, "lucet_globals_spec");
    err                                     = dlerror();
    if (err) {
        if (lucet_rep_err) {
            fprintf(stderr, "missing wasm globals spec. dlsym_error: %s\n", err);
        }
        goto error;
    }
    if (globals_spec == NULL) {
        if (lucet_rep_err) {
            fprintf(stderr, "null wasm globals spec\n");
        }
        goto error;
    }
    bool globals_valid = lucet_globals_validate(globals_spec);
    if (!globals_valid) {
        if (lucet_rep_err) {
            fprintf(stderr,
                    "globals spec invalid (may use an import global, which is not supported)\n");
        }
        goto error;
    }

    m->runtime_spec = (struct lucet_alloc_runtime_spec){
        .heap    = heap_spec,
        .globals = globals_spec,
    };
    assert(m->runtime_spec.heap);
    assert(m->runtime_spec.globals);

    m->trap_manifest = (struct lucet_trap_manifest){
        .records = dlsym(m->dl_handle, "lucet_trap_manifest"),
        .len     = 0,
    };
    err                      = dlerror();
    uint32_t *p_manifest_len = (uint32_t *) dlsym(m->dl_handle, "lucet_trap_manifest_len");
    char *    len_err        = dlerror();
    if (!err && !len_err) {
        // Both symbols found; all good
        m->trap_manifest.len = *p_manifest_len;
    } else if (!err && len_err) {
        if (lucet_rep_err) {
            fprintf(stderr,
                    "found lucet_trap_manifest sym but not lucet_trap_manifest_len "
                    "sym\n");
            fprintf(stderr, "dlsym error: %s\n", len_err);
        }
        goto error;
    } else if (err && !len_err) {
        if (lucet_rep_err) {
            fprintf(stderr,
                    "found lucet_trap_manifest_len sym but not lucet_trap_manifest "
                    "sym\n");
            fprintf(stderr, "dlsym error: %s\n", err);
        }
        goto error;
    } else {
        m->trap_manifest.len     = 0;
        m->trap_manifest.records = NULL;
    }

    if (heap_spec != NULL) {
        Dl_info dli;
        int     res = dladdr((void *) heap_spec, &dli);
        if (res > 0) {
            m->fbase = dli.dli_fbase;
        }
    }

    lucet_stats_update(lucet_stat_program_load, 1);
    return m;

error:
    if (m->dl_handle != NULL) {
        int ret = dlclose(m->dl_handle);
        if (ret != 0) {
            fprintf(stderr, "dlclose error: %d - %s\n", ret, dlerror());
        }
    }
    if (m != NULL) {
        free(m);
    }
    lucet_stats_update(lucet_stat_program_load_fail, 1);

    return NULL;
}

void lucet_module_unload(struct lucet_module *p)
{
    assert(p);
    if (dlclose(p->dl_handle) != 0) {
        fprintf(stderr, "dlclose error: %s\n", dlerror());
        exit(1);
    }
    free(p);
    lucet_stats_update(lucet_stat_program_unload, 1);
}

lucet_module_export_func *lucet_module_get_export_func(struct lucet_module const *m,
                                                       const char *               name)
{
    char symbol[256];
    int  res = snprintf(symbol, sizeof(symbol), "guest_func_%s", name);
    if (res < 0) {
        return NULL;
    }

    void *sym = dlsym(m->dl_handle, symbol);
    return sym;
}

lucet_module_export_func *lucet_module_get_start_func(struct lucet_module const *m)
{
    void *sym = dlsym(m->dl_handle, "guest_start");
    return sym;
}

void lucet_module_get_addr_details(struct lucet_module const *       m,
                                   struct lucet_module_addr_details *details, uintptr_t addr)
{
    assert(m);
    assert(details);

    Dl_info dli;
    int     res                     = dladdr((void *) addr, &dli);
    details->module_code_resolvable = res > 0;
    details->in_module_code         = res > 0 && dli.dli_fbase == m->fbase;
    // Note that these strings do not need to be freed
    details->file_name = dli.dli_fname;
    details->sym_name  = dli.dli_sname;
}

lucet_module_export_func *lucet_module_get_func_from_id(struct lucet_module const *m,
                                                        uint32_t table_id, uint32_t func_id)
{
    const struct lucet_table_element *element;

    if (table_id != 0U || m->table_elements.elements == NULL ||
        func_id > m->table_elements.last_id) {
        return NULL;
    }
    element = &(m->table_elements.elements[func_id]);
    return (lucet_module_export_func *) (void *) (uintptr_t) element->ref;
}
