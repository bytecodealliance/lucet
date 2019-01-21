#ifndef LUCET_MODULE_PRIVATE_H
#define LUCET_MODULE_PRIVATE_H

#include <dlfcn.h>
#include <stdint.h>

#include "lucet_alloc_private.h"
#include "lucet_data_segment_private.h"
#include "lucet_sparse_page_data_private.h"
#include "lucet_trap_private.h"

#pragma pack(push, 1)
struct lucet_table_element {
    uint64_t element_type;
    uint64_t ref;
};
#pragma pack(pop)

struct lucet_table_elements {
    struct lucet_table_element *elements;
    uint32_t                    last_id;
};

// an `lucet_module` represents the contents of a native-compiled wasm module
// loaded from a dynamic library.
struct lucet_module {
    // Handle given to us by dlopen, which we need to close upon unload.
    void *dl_handle;
    // Base address of the dynamically loaded code
    void *fbase;

    // Valid data segment initializer
    struct lucet_data_segment_descriptor data_segment;

    // Spec for heap and globals required by module
    struct lucet_alloc_runtime_spec runtime_spec;

    // Manifest of trap site tables
    struct lucet_trap_manifest trap_manifest;

    struct lucet_sparse_page_data *sparse_page_data;

    struct lucet_table_elements table_elements;
};

typedef void lucet_module_export_func(void *);

lucet_module_export_func *lucet_module_get_export_func(struct lucet_module const *m,
                                                       const char *               name);

lucet_module_export_func *lucet_module_get_start_func(struct lucet_module const *m);

lucet_module_export_func *lucet_module_get_func_from_id(struct lucet_module const *m,
                                                        uint32_t table_id, uint32_t func_id);

#endif // LUCET_MODULE_PRIVATE_H
