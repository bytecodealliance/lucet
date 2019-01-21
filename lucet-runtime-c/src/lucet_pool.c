#include <assert.h>
#include <bsd/sys/queue.h>
#include <err.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

#include "lucet_alloc_private.h"
#include "lucet_pool_private.h"

struct lucet_pool_entry {
    struct lucet_alloc *alloc;
    TAILQ_ENTRY(lucet_pool_entry) next;
};

struct lucet_pool {
    TAILQ_HEAD(, lucet_pool_entry) free;
    struct lucet_pool_entry *  entries;
    int                        allocated;
    int                        available;
    int                        refcount;
    pthread_mutex_t            lock;
    struct lucet_alloc_limits  limits;
    struct lucet_alloc_region *region;
};

// forward decls
static void lucet_pool_lock(struct lucet_pool *);
static void lucet_pool_unlock(struct lucet_pool *);
static int  lucet_pool_maybe_destroy(struct lucet_pool *);
static void lucet_pool_destroy(struct lucet_pool *);

struct lucet_pool *lucet_pool_create(int num_entries, struct lucet_alloc_limits const *limits)
{
    struct lucet_pool *pool = calloc(sizeof(struct lucet_pool), 1);
    if (pool == NULL) {
        err(1, "%s() failed to allocate lucet_pool", __FUNCTION__);
    }

    *pool = (struct lucet_pool){
        .allocated = num_entries,
        .available = num_entries,
        .refcount  = 1,
        .free      = TAILQ_HEAD_INITIALIZER(pool->free),
    };

    if (limits) {
        pool->limits = *limits;
    } else {
        pool->limits = (struct lucet_alloc_limits){
            .heap_memory_size        = 16 * 64 * 1024,  // 16 wasm pages
            .heap_address_space_size = 8 * 1024 * 1024, // 8mb total (4mb reserved + 4mb guard)
            .stack_size              = 128 * 1024,
            .globals_size            = 4096,
        };
    }

    if (pthread_mutex_init(&pool->lock, NULL) != 0) {
        err(1, "%s() failed to initialize lock", __FUNCTION__);
    }

    pool->entries = calloc(sizeof(struct lucet_pool_entry), num_entries);
    if (pool->entries == NULL) {
        err(1, "%s() failed to allocate pool entries", __FUNCTION__);
    }

    pool->region = lucet_alloc_create_region(num_entries, &pool->limits);

    for (int i = 0; i < num_entries; i++) {
        pool->entries[i].alloc = lucet_alloc_region_get_alloc(pool->region, i);
        assert(pool->entries[i].alloc != NULL);
        TAILQ_INSERT_TAIL(&pool->free, &pool->entries[i], next);
    }
    return pool;
}

void lucet_pool_incref(struct lucet_pool *pool)
{
    lucet_pool_lock(pool);

    pool->refcount++;

    lucet_pool_unlock(pool);
}

void lucet_pool_decref(struct lucet_pool *pool)
{
    lucet_pool_lock(pool);

    assert(pool->refcount > 0);
    pool->refcount--;

    if (lucet_pool_maybe_destroy(pool) != 0) {
        lucet_pool_unlock(pool);
    }
}

static int lucet_pool_maybe_destroy(struct lucet_pool *pool)
{
    if (pool->refcount == 0 && pool->available == pool->allocated) {
        // At this point we are certain that no one else has a reference so
        // we can unlock it and begin destruction.
        lucet_pool_unlock(pool);
        lucet_pool_destroy(pool);
        return 0;
    }
    return 1;
}

static void lucet_pool_destroy(struct lucet_pool *pool)
{
    assert(pool->refcount == 0);
    assert(pool->available == pool->allocated);

    if (pthread_mutex_destroy(&pool->lock) != 0) {
        err(1, "%s() failed to destroy lock", __FUNCTION__);
    }

    free(pool->entries);
    free(pool);
}

struct lucet_alloc *lucet_pool_acquire(struct lucet_pool *pool)
{
    lucet_pool_lock(pool);

    // Any callers of lucet_pool_acquire (via lucet_instantiate, presumably)
    // should be holding a reference to the pool. Thus this should never be
    // zero.
    assert(pool->refcount > 0);

    if (pool->available == 0) {
        // There are no more instance_mems available in this pool.
        goto error;
    }

    struct lucet_pool_entry *entry;
    entry = TAILQ_FIRST(&pool->free);
    assert(entry); // Per check above

    TAILQ_REMOVE(&pool->free, entry, next);

    assert(pool->available > 0);
    pool->available--;

    lucet_pool_unlock(pool);
    return entry->alloc;

error:
    lucet_pool_unlock(pool);
    return NULL;
}

void lucet_pool_release(struct lucet_pool *pool, struct lucet_alloc *a)
{
    assert(a);
    lucet_pool_lock(pool);

    assert(pool->available < pool->allocated);

    // Find the pool entry to push onto the free list:
    struct lucet_pool_entry *entry = NULL;
    for (int i = 0; i < pool->allocated; i++) {
        if (pool->entries[i].alloc == a) {
            entry = &pool->entries[i];
            break;
        }
    }

    assert(entry != NULL);

    if (pool->refcount == 0) {
        assert(pool->allocated > 0);
        pool->allocated--;
    } else {
        TAILQ_INSERT_TAIL(&pool->free, entry, next);
        pool->available++;
    }

    lucet_pool_unlock(pool);
}

static void lucet_pool_lock(struct lucet_pool *pool)
{
    if (pthread_mutex_lock(&pool->lock) != 0) {
        err(1, "%s() failed to lock pool", __FUNCTION__);
    }
}

static void lucet_pool_unlock(struct lucet_pool *pool)
{
    if (pthread_mutex_unlock(&pool->lock) != 0) {
        err(1, "%s() failed to unlock pool", __FUNCTION__);
    }
}
