
#include <stddef.h>
#include <stdbool.h>
#include <errno.h>

#include "lend.h"

void *__expand_heap(size_t *);

static void try_grow(size_t bytes) {
	// There are 24 bytes of bookkeeping required to add a new region to the
	// heap, and allocate it.
	size_t bytes_with_overhead = bytes + 24;
	void* area = __expand_heap(&bytes_with_overhead);
	if (area) {
		lend_give(area, bytes_with_overhead);
	}
}

void *malloc(size_t n) {
	void* res = lend_malloc(n);
	if (res == NULL) {
		try_grow(n);
		void* res2 = lend_malloc(n);
		if (res2 == NULL) {
			errno = ENOMEM;
		}
		return res2;
	} else {
		return res;
	}
}

void* calloc(size_t numb, size_t size) {
	void* res = lend_calloc(numb, size);
	if (res == NULL) {
		try_grow(numb * size);
		void* res2 = lend_calloc(numb, size);
		if (res2 == NULL) {
			errno = ENOMEM;
		}
		return res2;
	} else {
		return res;
	}
}

void *realloc(void *p, size_t n) {
	if (n == 0) {
		lend_free(p);
		return NULL;
	} else {
		void* res = lend_realloc(p, n);
		if (res == NULL) {
			try_grow(n);
			void* res2 = lend_realloc(p, n);
			if (res2 == NULL) {
				errno = ENOMEM;
			}
			return res2;
		} else {
			return res;
		}
	}
}

void free(void *objp) {
	lend_free(objp);
}
