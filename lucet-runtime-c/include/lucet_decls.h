#ifndef LUCET_DECLS_H
#define LUCET_DECLS_H

// We need these declarations in nearly every header, so they are put all in one
// place.  All of these structs are opaque to the library user. They are defined
// in lucet_private.h
struct lucet_module;
struct lucet_instance;
struct lucet_pool;
struct lucet_vmctx;

#endif // LUCET_DECLS_H
