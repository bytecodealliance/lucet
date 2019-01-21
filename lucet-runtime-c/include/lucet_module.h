/**
 * @file lucet_module.h
 * @brief Functions related to loading and unloading modules.
 */

#ifndef LUCET_MODULE_H
#define LUCET_MODULE_H

#include <stdbool.h>
#include <stdint.h>

#include "lucet_decls.h"
#include "lucet_export.h"

/**
 * Create a module, loading the code from a shared object in the filesystem.
 * Invokes the linker through dlopen.
 *
 * Returns NULL on failure.
 */
struct lucet_module *lucet_module_load(const char *dl_path) EXPORTED;

/**
 * Frees memory allocated in `lucet_module_load`, and unloads the code.
 */
void lucet_module_unload(struct lucet_module *) EXPORTED;

/**
 * Details about a program address.
 * It is possible to determine whether an address lies within the module code if
 * the module is loaded from a shared object. Statically linked modules are not
 * resolvable.
 * Best effort is made to resolve the symbol the address is found inside, and
 * the file that symbol is found in. See dladdr(3) for more details.
 */
struct lucet_module_addr_details {
    bool        module_code_resolvable;
    bool        in_module_code;
    const char *file_name;
    const char *sym_name;
};

/**
 * Enables full reporting of internal errors within lucet_module_load.
 * Returns prior state - default state is FALSE.
 */
bool lucet_report_load_errors(bool flag) EXPORTED;

/**
 * Resolve details of a given instruction address (ip).
 */
void lucet_module_get_addr_details(struct lucet_module const *       m,
                                   struct lucet_module_addr_details *details, uintptr_t ip);

#endif // LUCET_MODULE_H
