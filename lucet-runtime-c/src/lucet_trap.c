#include <assert.h>
#include <err.h>
#include <stdio.h>

#include "lucet_probestack_private.h"
#include "lucet_trap_private.h"

struct lucet_trapcode lucet_trapcode_deserialize(uint32_t trapcode_bin)
{
    struct lucet_trapcode trapcode = (struct lucet_trapcode){
        .code = (enum lucet_trapcode_type)(trapcode_bin & 0x0000FFFF),
        .tag  = (uint16_t)((trapcode_bin & 0xFFFF0000) >> 16),
    };
    return trapcode;
}

const char *lucet_trapcode_type_string(int trapcode)
{
    switch (trapcode) {
    case lucet_trapcode_stack_overflow:
        return "stack overflow";
    case lucet_trapcode_heap_oob:
        return "heap out-of-bounds";
    case lucet_trapcode_oob:
        return "out-of-bounds";
    case lucet_trapcode_indirect_call_to_null:
        return "indirect call to null";
    case lucet_trapcode_bad_signature:
        return "bad signature";
    case lucet_trapcode_integer_overflow:
        return "integer overflow";
    case lucet_trapcode_integer_div_by_zero:
        return "division by zero";
    case lucet_trapcode_bad_conversion_to_integer:
        return "bad conversion to integer";
    case lucet_trapcode_interrupt:
        return "interrupt";
    case lucet_trapcode_table_out_of_bounds:
        return "table out of bounds";
    case lucet_trapcode_user:
        return "user";
    case lucet_trapcode_unknown:
        return "unknown";
    default:
        errx(1, "%s() unexpected trap code", __FUNCTION__);
    }
}

int lucet_trapcode_display(char *str, size_t len, struct lucet_trapcode const *trapcode)
{
    if (trapcode->code == lucet_trapcode_user) {
        return snprintf(str, len, "%s(%u)", lucet_trapcode_type_string(trapcode->code),
                        trapcode->tag);
    } else {
        return snprintf(str, len, "%s", lucet_trapcode_type_string(trapcode->code));
    }
}

struct lucet_trapcode lucet_trap_lookup(const struct lucet_trap_manifest *manifest, uintptr_t rip)
{
    assert(manifest->records != NULL);

    struct lucet_trap_manifest_record const *record = manifest->records;
    for (int i = 0; i < manifest->len; i++) {
        uintptr_t func_addr = (uintptr_t) record->func_addr;

        if (rip >= func_addr && rip <= (func_addr + record->func_len)) {
            // The trap has fallen within a known function!
            struct lucet_trap_trapsite *trapsite =
                (struct lucet_trap_trapsite *) record->table_addr;

            // TODO: Note that this could be turned into a binary
            //       search. (The top level scan can't though.)
            for (int j = 0; j < record->table_len; j++) {
                if (rip == func_addr + trapsite->offset) {
                    // It's a (safe) trap!
                    return lucet_trapcode_deserialize(trapsite->trapcode);
                }
                trapsite++;
            }

            // Exit early. Only one function should ever match.
            break;
        }
        record++;
    }

    // Special case: lucet_probestack is not in the guest shared object, but is
    // called to test each page that a greater-than-page-size stack frame is
    // about to expand into
    if (rip >= (uintptr_t) &lucet_probestack &&
        rip <= ((uintptr_t) &lucet_probestack + lucet_probestack_size)) {
        return (struct lucet_trapcode){
            .code = lucet_trapcode_stack_overflow, .tag = UINT16_MAX,
            // Special tag just for testing. Ordinarily, stack overflow traps
            // don't get a tag, so it shouldn't be inspected.
        };
    }

    // We did not find a trapcode, so we construct an "unknown" one.
    struct lucet_trapcode unknown = (struct lucet_trapcode){
        .code = lucet_trapcode_unknown,
        .tag  = 0,
    };
    return unknown;
}
