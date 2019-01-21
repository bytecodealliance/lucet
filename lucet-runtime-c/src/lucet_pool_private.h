
#ifndef LUCET_POOL_PRIVATE_H
#define LUCET_POOL_PRIVATE_H

#include "lucet_alloc_private.h"
#include "lucet_pool.h"

// Get an alloc from the pool
struct lucet_alloc *lucet_pool_acquire(struct lucet_pool *);

// Return an alloc to the pool
void lucet_pool_release(struct lucet_pool *, struct lucet_alloc *);

#endif // LUCET_POOL_PRIVATE_H
