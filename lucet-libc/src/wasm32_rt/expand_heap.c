#include <stdint.h>
#include <stddef.h>
#include <limits.h>
#include <errno.h>

void *__expand_heap(size_t *pn)
{
	size_t page_size = 64 * 1024;
	// Translate from byte size to page size. Round up to nearest page:
	size_t new_pages = (*pn + (page_size - 1)) / page_size;
	// Returns either the base page of the new area, or -1 on failure.
	int32_t new_base = __builtin_wasm_grow_memory(new_pages);

	if (new_base == -1) {
		*pn = 0;
		errno = ENOMEM;
		return NULL;
	}

	// Translate back from pages to pointers
	void* area = (void*) ((size_t)new_base * page_size);
	*pn = new_pages * page_size;
	return area;
}
