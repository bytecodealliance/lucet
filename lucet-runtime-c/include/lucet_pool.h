#ifndef LUCET_POOL_H
#define LUCET_POOL_H

#include "lucet_alloc.h"
#include "lucet_decls.h"
#include "lucet_export.h"

// Create a new pool. Pools store memory allocations for lucet_instances. These
// allocations are not accessible from the public interface.
// `lucet_instance_create` uses a private interface to retrieve the allocation it
// needs.
//
// Arguments:
// `entries` specifies how many instance allocations are created.
// `limits` specifies the sizes of the heap, guard page, and stack reserved for
// those instances. Instances requiring a smaller heap or guard may be created,
// but larger ones will be rejected at `lucet_instance_create`. Requirements are
// found in the `lucet_alloc_heap_spec` that is a private member of the
// lucet_module.
// If `limits` is NULL, a set of defaults is used instead.
//
// When you create a pool it has a refcount of 1. If you need to start using
// that pool from other callers, they need to hold their own references. Once
// the ref count hits zero the pool will begin draining itself and when all
// outstanding instance_mems have been returned, it will free itself. (As, at
// that point, no one should have a reference to it anymore.)
//
// Pools are thread-safe and may be used from multiple threads simultaneously.
struct lucet_pool *lucet_pool_create(int entries, struct lucet_alloc_limits const *limits) EXPORTED;

// Take an additional reference to an lucet_pool.
//
// If you need to instantiate lucet_instances from multiple entities, each one
// will need a reference. See lucet_pool_create.
void lucet_pool_incref(struct lucet_pool *) EXPORTED;

// Return a reference to a pool.
//
// To destroy a pool, all references must be returned. Once all references and
// all outstanding instances have been returned it will deallocate itself.
void lucet_pool_decref(struct lucet_pool *) EXPORTED;

#endif // LUCET_POOL_H
