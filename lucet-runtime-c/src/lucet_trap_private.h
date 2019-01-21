#ifndef LUCET_TRAP_PRIVATE_H
#define LUCET_TRAP_PRIVATE_H 1

#include "lucet_trap.h"

struct lucet_trap_manifest {
    uint32_t                                 len;
    struct lucet_trap_manifest_record const *records;
};

struct lucet_trap_manifest_record {
    uint64_t func_addr;
    uint64_t func_len;
    uint64_t table_addr;
    uint64_t table_len;
};

struct lucet_trap_trapsite {
    uint32_t offset;
    uint32_t trapcode;
};

int lucet_trapcode_display(char *str, size_t len, struct lucet_trapcode const *trapcode);

/* Looks up an instruction pointer in a trap manifest.
 * If it finds a matching trapsite, returns the serialized trapcode as a uint32_t.
 * If it does not find a matching trapsite, return ~0 as a uint32_t.
 *
 * Note: lucet_trap_lookup is specifically intended to be signal-safe.
 */
struct lucet_trapcode lucet_trap_lookup(const struct lucet_trap_manifest *manifest, uintptr_t rip);

/* Deserialized a packed trapcode into a `struct lucet_trapcode`.
 */
struct lucet_trapcode lucet_trapcode_deserialize(uint32_t trapcode_bin);

#endif
