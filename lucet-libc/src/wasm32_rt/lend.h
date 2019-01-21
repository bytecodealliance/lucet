#ifndef _LEND_H
#define _LEND_H

#include <stddef.h>

void lend_give(void *addr, size_t size);
void lend_show(void);
void *lend_malloc(size_t size);
void *lend_calloc(size_t numb, size_t size);
void *lend_realloc(void *oldp, size_t size);
void lend_free(void *objp);

#endif
