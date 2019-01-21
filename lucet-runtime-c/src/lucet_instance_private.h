
#ifndef LUCET_INSTANCE_PRIVATE_H
#define LUCET_INSTANCE_PRIVATE_H

#include <stddef.h>

#include "lucet_context_private.h"
#include "lucet_instance.h"
#include "lucet_state.h"
#include "lucet_vmctx_private.h"

#define LUCET_INSTANCE_MAGIC 746932922 // gensym

// specifies an instance of a sandbox
struct lucet_instance {
    // Used to catch bugs in pointer math used to find address of instance:
    uint64_t magic;
    // The delegate object is a pointer from the embedder that is used in host
    // calls
    void *delegate_obj;
    // The program is an entry point for the instance. Many instances can be
    // run on one program simultaneously.
    struct lucet_module const *module;
    // The pool is a reference back to the pool which allocated this instance.
    struct lucet_pool *pool;
    // We use lucet_context for a cooperative stack to run the guest program on
    struct lucet_context ctx;

    // Error information:
    struct lucet_state state;

    // Tracks allocation of this structure and the heap that follows it.
    struct lucet_alloc *alloc;

    // Fatal handler used when an instance exits in a fatal state
    lucet_fatal_handler *fatal_handler;

    // Signal handler used to interpret unhandled signals
    lucet_signal_handler *signal_handler;

    // Pointer to function used as the entrypoint, for use in backtraces
    void *entrypoint;

    // Spacer to ensure globals pointer is precisely at the end of the instance
    // struct.
    char _reserved[2488];

    // Pointer to globals. This is accessed through the vmctx, which points to
    // the heap, which is immediately after this struct.
    char *globals;
};

// These static assertions ensure that the lucet_instance struct is laid out so
// that the globals pointer can be accessed as vmctx[-8].
_Static_assert((sizeof(struct lucet_instance) == 4096),
               "lucet instance struct exactly 1 page long");
_Static_assert((offsetof(struct lucet_instance, globals) == (4096 - 8)),
               "globals pointer precisely at end of instance");

// Get the instance corresponding to a vmctx
struct lucet_instance *lucet_vmctx_instance(struct lucet_vmctx const *);
// And vice versa
struct lucet_vmctx const *lucet_instance_vmctx(struct lucet_instance const *i);

// Terminate an instance, from inside a host call. for use from vmctx.
void lucet_instance_terminate(struct lucet_instance *, void *);

#endif // LUCET_INSTANCE_PRIVATE_H
