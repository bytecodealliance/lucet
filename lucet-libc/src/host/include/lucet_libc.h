#ifndef LUCET_LIBC_H
#define LUCET_LIBC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#define LUCET_LIBC_EXPORTED __attribute__((visibility("default")))

struct lucet_libc;

typedef void lucet_libc_stdio_handler(struct lucet_libc *, int fd, const char* str, size_t len);

enum lucet_libc_term_reason {
	lucet_libc_term_none = 0,
	lucet_libc_term_exit,
	lucet_libc_term_abort,
	lucet_libc_term_check_heap,
};

struct lucet_libc {
	uint64_t magic;
	enum lucet_libc_term_reason term_reason;
	union {
		int32_t exit;
		const char* check_heap;
	} term_info;
	lucet_libc_stdio_handler *stdio_handler;
};

/**
 * Initialize an lucet_libc
 */

void lucet_libc_init(struct lucet_libc *) LUCET_LIBC_EXPORTED;

/**
 * Did the libc handle a termination of the sandbox?
 */
bool lucet_libc_terminated(struct lucet_libc const *) LUCET_LIBC_EXPORTED;

/**
 * Did libc termination happen via the abort syscall?
 */
bool lucet_libc_aborted(struct lucet_libc const *) LUCET_LIBC_EXPORTED;

/**
 * Did libc termination happen via the exit syscall?
 */
bool lucet_libc_exited(struct lucet_libc const *) LUCET_LIBC_EXPORTED;

/**
 * What was the exit syscall argument? Precondition: lucet_libc_exited is true.
 */
int32_t lucet_libc_exit_code(struct lucet_libc const *) LUCET_LIBC_EXPORTED;

/**
 * Set a stdio handler function for the libc.
 */
void lucet_libc_set_stdio_handler(struct lucet_libc *, lucet_libc_stdio_handler*) LUCET_LIBC_EXPORTED;

const char* lucet_libc_term_reason_str(int lucet_libc_term_reason) LUCET_LIBC_EXPORTED;

#endif // LUCET_LIBC_H
