#include "lucet_vmctx.h"
#include "session.h"

// In the header (which is deliberately NOT imported), the signature of
// get_header is (const char* key, size_t key_len, char* value, size_t*
// value_len). All pointers have been translated into offsets into the heap by
// the caller code, and the size_t key_len has been translated to a u32 (size_t
// is u64 on x86_64) because we are emulating the wasm32 runtime where size_t is
// a u32.
void session_hostcall_get_header(struct lucet_vmctx *ctx, guest_ptr_t key_ptr, guest_size_t key_len,
                                 guest_ptr_t value_ptr, guest_ptr_t value_len_ptr)
{
    char *          heap = lucet_vmctx_get_heap(ctx);
    struct session *s    = (struct session *) lucet_vmctx_get_delegate(ctx);
    // Syscall args are:
    // const unsigned char *key
    // size_t key_len
    // unsigned char *val
    // size_t *val_len
    const unsigned char *key     = (const unsigned char *) &heap[key_ptr];
    unsigned char *      val     = (unsigned char *) &heap[value_ptr];
    guest_size_t *       val_len = (guest_size_t *) &heap[value_len_ptr];
    if (lucet_vmctx_check_heap(ctx, (void *) key, key_len) &&
        lucet_vmctx_check_heap(ctx, (void *) val_len, sizeof(guest_size_t)) &&
        lucet_vmctx_check_heap(ctx, (void *) val, *val_len)) {
        session_get_header(s, key, key_len, val, val_len);
    } else {
        lucet_vmctx_terminate(ctx, (void *) "session_hostcall_get_header check_heap failed");
    }
}

// in the header, the signature of send is (const char* buf, size_t buf_len).
// The buf pointer has been translated into a heap offset by caller, and size_t
// is restricted to 32 bits to emulate wasm32.
void session_hostcall_send(struct lucet_vmctx *ctx, guest_ptr_t buf_ptr, guest_size_t buf_len)
{
    char *          heap = lucet_vmctx_get_heap(ctx);
    struct session *s    = (struct session *) lucet_vmctx_get_delegate(ctx);
    // Syscall args are:
    // const unsigned char *buf
    // size_t buf_len
    const unsigned char *buf = (const unsigned char *) &heap[buf_ptr];
    if (lucet_vmctx_check_heap(ctx, (void *) buf, buf_len)) {
        session_send(s, buf, buf_len);
    } else {
        lucet_vmctx_terminate(ctx, (void *) "session_hostcall_send check_heap failed");
    }
}
