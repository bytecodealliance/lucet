#include <assert.h>
#include "lucet_vmctx.h"
#include "lucet_libc.h"

#define LUCET_LIBC_MAGIC 0x5fa3139c52ULL

/**
 * Hostcall
 */
void lucet_libc_stdio(struct lucet_vmctx const *ctx, int32_t fd, guest_ptr_t buf_ptr, guest_size_t len) {
	struct lucet_libc *libc = (struct lucet_libc *) lucet_vmctx_get_delegate(ctx);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	const char* heap = lucet_vmctx_get_heap(ctx);
	const char* buf = &heap[buf_ptr];
	if (lucet_vmctx_check_heap(ctx, (void*) buf, (size_t) len)) {
		if (libc->stdio_handler) {
			libc->stdio_handler(libc, fd, buf, (size_t) len);
		}
    } else {
		libc->term_reason = lucet_libc_term_check_heap;
		libc->term_info.check_heap = "lucet_libc_stdio";
		lucet_vmctx_terminate(ctx, NULL);
	}
}

/**
 * Hostcall
 */
void lucet_libc_syscall_abort(struct lucet_vmctx const *ctx) {
	struct lucet_libc *libc = (struct lucet_libc *) lucet_vmctx_get_delegate(ctx);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	libc->term_reason = lucet_libc_term_abort;
	lucet_vmctx_terminate(ctx, NULL);
}

/**
 * Hostcall
 */
void lucet_libc_syscall_exit(struct lucet_vmctx const *ctx, int32_t code) {
	struct lucet_libc *libc = (struct lucet_libc *) lucet_vmctx_get_delegate(ctx);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	libc->term_reason = lucet_libc_term_exit;
	libc->term_info.exit = code;
	lucet_vmctx_terminate(ctx, NULL);
}

void lucet_libc_init(struct lucet_libc * libc) {
	/* Force the hostcall functions to be linked in. */
	(void) lucet_libc_stdio;
	(void) lucet_libc_syscall_abort;
	(void) lucet_libc_syscall_exit;

	libc->magic = LUCET_LIBC_MAGIC;
}

/**
 * Host API
 */
bool lucet_libc_terminated(struct lucet_libc const * libc) {
	assert(libc);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	return (libc->term_reason != lucet_libc_term_none);
}

/**
 * Host API
 */
bool lucet_libc_aborted(struct lucet_libc const * libc) {
	assert(libc);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	return (libc->term_reason == lucet_libc_term_abort);
}

/**
 * Host API
 */
bool lucet_libc_exited(struct lucet_libc const * libc) {
	assert(libc);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	return (libc->term_reason != lucet_libc_term_none);
}

/**
 * Host API
 */
int32_t lucet_libc_exit_code(struct lucet_libc const * libc) {
	assert(libc);
	assert(libc->term_reason == lucet_libc_term_exit);
	return libc->term_info.exit;
}

/**
 * Host API
 */
void lucet_libc_set_stdio_handler(struct lucet_libc *libc, lucet_libc_stdio_handler *handler) {
	assert(libc);
	assert(libc->magic == LUCET_LIBC_MAGIC);
	libc->stdio_handler = handler;
}

const char* lucet_libc_term_reason_str(int reason) {
	switch((enum lucet_libc_term_reason) reason) {
		case lucet_libc_term_none:
			return "none";
		case lucet_libc_term_exit:
			return "exit";
		case lucet_libc_term_abort:
			return "abort";
		case lucet_libc_term_check_heap:
			return "check heap";
		default:
			return "<invalid>";
	}
}
