#ifndef LUCET_EXPORT_H
#define LUCET_EXPORT_H

// liblucet can be built into a shared object, with cflags that hide every symbol
// that does not declare its visibility to be default. For the symbols that are
// part of the public API for this library, we add this attribute:
#define EXPORTED __attribute__((visibility("default")))

#endif // LUCET_EXPORT_H
