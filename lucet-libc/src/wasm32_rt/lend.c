#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <inttypes.h>
#include "lend.h"

struct meta {
    uintptr_t    magi;
    size_t       size;
    struct meta *next;
};

#define LIST(x) ((struct meta *)(x) - 1)
#define DATA(x) ((uintptr_t)((struct meta*)(x) + 1))
#define BUSY    ((uintptr_t)0xFEEDFEEDFEEDFEEDU)
#define IDLE    ((uintptr_t)0xDEADDEADDEADDEADU)
#define FILL    sizeof(struct meta) /* no free fragments smaller than this */

static struct meta *root;

void *lend_malloc(size_t size) {
    size_t const orig_size = size;
    size += FILL - (size % FILL); /* round up */
    /* check for overflow */
    if (size < orig_size) return NULL;

    for(struct meta *list = root; list; list = list->next) {
        if(list->magi == BUSY) {
            continue;
        } else if(list->magi == IDLE) {
            while(list->next && list->next->magi == IDLE
              && (uintptr_t) list->next == DATA(list) + list->size) {
                /* Join */
                list->size += list->next->size + sizeof(struct meta);
                list->next->magi = 0;
                list->next = list->next->next;
            }

            if(list->size < size)
                continue;

            if(list->size > size + sizeof(struct meta) + FILL) {
                /* Fork */
                struct meta *newl = (struct meta*)(DATA(list) + size);
                newl->magi = IDLE;
                newl->size = list->size - size - sizeof(struct meta);
                newl->next = list->next;
                list->next = newl;
                list->size = size;
            }
            list->magi = BUSY;
            return (void*)DATA(list);
        } else {
            printf("lend_malloc(): heap corruption detected at 0x%"PRIxPTR"\n", (uintptr_t) list);
            abort();
        }
    }

    return NULL;
}

void *lend_calloc(size_t numb, size_t size) {
    size_t objs = numb * size;

    /* Overflow check */
#define HALF_SIZE_T (((size_t) 1) << (8 * sizeof(size_t) / 2))
    if(__builtin_expect((numb | size) >= HALF_SIZE_T, 0))
        if(size != 0 && objs / size != numb)
            return NULL;

    void *objp = lend_malloc(objs);
    if(objp)
        memset(objp, 0, objs);

    return objp;
}

void *lend_realloc(void *oldp, size_t size) {
    void *newp = lend_malloc(size);

    if(oldp && newp) {
        size_t olds;
        if(size > LIST(oldp)->size)
            olds = LIST(oldp)->size;
        else
            olds = size;

        memcpy(newp, oldp, olds);
        lend_free(oldp);
    }

    return newp;
}

void lend_free(void *objp) {
    if(objp == NULL)
        return;

    struct meta *list = LIST(objp);
    if(list->magi != BUSY) {
        printf("lend_free(): heap corruption detected at 0x%"PRIxPTR"\n", (uintptr_t) list);
        abort();
    }
    list->magi = IDLE;
}

void lend_give(void *area, size_t size) {
    if(size < sizeof(struct meta) + FILL)
        return;

    struct meta *list = (struct meta *)area;
    list->magi = IDLE;
    list->size = size - sizeof(struct meta);
    list->next = root;
    root = list;
}

void lend_show(void) {
    size_t busy = 0, idle = 0, meta = 0;

    printf("Heap view:\n");

    for(struct meta *list = root; list; list = list->next) {
        meta += sizeof(struct meta);

        const char *magi;
        switch(list->magi) {
        case IDLE: magi = "IDLE"; idle += list->size; break;
        case BUSY: magi = "BUSY"; busy += list->size; break;
        default:   magi = "!!!!";
        }

        printf("%s 0x%"PRIxPTR" + 0x%zx -> 0x%"PRIxPTR"\n",
               magi, (uintptr_t)list, list->size, (uintptr_t) list->next);
        if(list->magi != BUSY && list->magi != IDLE)
            return;
    }

    printf(" === busy: 0x%zx idle: 0x%zx meta: 0x%zx full: 0x%zx\n",
           busy, idle, meta, busy + idle + meta);
}
