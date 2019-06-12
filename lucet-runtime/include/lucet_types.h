#ifndef LUCET_TYPES_H
#define LUCET_TYPES_H

#ifndef _XOPEN_SOURCE
# define _XOPEN_SOURCE 500
#endif

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __APPLE__
# include <sys/ucontext.h>
#else
# include <ucontext.h>
#endif

enum lucet_error {
    lucet_error_ok,
    lucet_error_invalid_argument,
    lucet_error_region_full,
    lucet_error_module,
    lucet_error_limits_exceeded,
    lucet_error_no_linear_memory,
    lucet_error_symbol_not_found,
    lucet_error_func_not_found,
    lucet_error_runtime_fault,
    lucet_error_runtime_terminated,
    lucet_error_dl,
    lucet_error_internal,
    lucet_error_unsupported,
};

enum lucet_signal_behavior {
    lucet_signal_behavior_default,
    lucet_signal_behavior_continue,
    lucet_signal_behavior_terminate,
};

enum lucet_state_tag {
    lucet_state_tag_returned,
    lucet_state_tag_running,
    lucet_state_tag_fault,
    lucet_state_tag_terminated,
};

enum lucet_terminated_reason {
    lucet_terminated_reason_signal,
    lucet_terminated_reason_get_embed_ctx,
    lucet_terminated_reason_provided,
};

enum lucet_trapcode_type {
    lucet_trapcode_type_stack_overflow,
    lucet_trapcode_type_heap_out_of_bounds,
    lucet_trapcode_type_out_of_bounds,
    lucet_trapcode_type_indirect_call_to_null,
    lucet_trapcode_type_bad_signature,
    lucet_trapcode_type_integer_overflow,
    lucet_trapcode_type_integer_div_by_zero,
    lucet_trapcode_type_bad_conversion_to_integer,
    lucet_trapcode_type_interrupt,
    lucet_trapcode_type_table_out_of_bounds,
    lucet_trapcode_type_user,
    lucet_trapcode_type_unknown,
};

enum lucet_val_type {
    lucet_val_type_c_ptr,
    lucet_val_type_guest_ptr,
    lucet_val_type_u8,
    lucet_val_type_u16,
    lucet_val_type_u32,
    lucet_val_type_u64,
    lucet_val_type_i8,
    lucet_val_type_i16,
    lucet_val_type_i32,
    lucet_val_type_i64,
    lucet_val_type_usize,
    lucet_val_type_isize,
    lucet_val_type_bool,
    lucet_val_type_f32,
    lucet_val_type_f64,
};

union lucet_val_inner_val {
    void *   as_c_ptr;
    uint64_t as_u64;
    int64_t  as_i64;
    float    as_f32;
    double   as_f64;
};

struct lucet_val {
    enum lucet_val_type       ty;
    union lucet_val_inner_val inner_val;
};

struct lucet_dl_module;

struct lucet_instance;

struct lucet_region;

/**
 * Runtime limits for the various memories that back a Lucet instance.
 * Each value is specified in bytes, and must be evenly divisible by the host page size (4K).
 */
struct lucet_alloc_limits {
    /**
     * Max size of the heap, which can be backed by real memory. (default 1M)
     */
    uint64_t heap_memory_size;
    /**
     * Size of total virtual memory. (default 8G)
     */
    uint64_t heap_address_space_size;
    /**
     * Size of the guest stack. (default 128K)
     */
    uint64_t stack_size;
    /**
     * Size of the globals region in bytes; each global uses 8 bytes. (default 4K)
     */
    uint64_t globals_size;
};

struct lucet_trapcode {
    enum lucet_trapcode_type code;
    uint16_t                 tag;
};

typedef enum lucet_signal_behavior (*lucet_signal_handler)(struct lucet_instance *      inst,
                                                           const struct lucet_trapcode *trap,
                                                           int signum, const siginfo_t *siginfo,
                                                           const void *context);

typedef void (*lucet_fatal_handler)(struct lucet_instance *inst);

struct lucet_untyped_retval {
    char fp[16];
    char gp[8];
};

struct lucet_module_addr_details {
    bool        module_code_resolvable;
    bool        in_module_code;
    const char *file_name;
    const char *sym_name;
};

struct lucet_runtime_fault {
    bool                             fatal;
    struct lucet_trapcode            trapcode;
    uintptr_t                        rip_addr;
    struct lucet_module_addr_details rip_addr_details;
    siginfo_t                        signal_info;
    ucontext_t                       context;
};

struct lucet_terminated {
    enum lucet_terminated_reason reason;
    void *                       provided;
};

union lucet_state_val {
    struct lucet_untyped_retval returned;
    bool                        running;
    struct lucet_runtime_fault  fault;
    struct lucet_terminated     terminated;
};

struct lucet_state {
    enum lucet_state_tag  tag;
    union lucet_state_val val;
};

union lucet_retval_gp {
    char     as_untyped[8];
    void *   as_c_ptr;
    uint64_t as_u64;
    int64_t  as_i64;
};

#endif /* LUCET_TYPES_H */
