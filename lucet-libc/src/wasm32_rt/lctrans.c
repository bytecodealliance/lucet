#include "locale_impl.h"

const char *__lctrans(const char *msg, const struct __locale_map *lm) {
	(void) lm;
	return msg;
}
