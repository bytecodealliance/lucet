
#ifndef LUCET_H
#define LUCET_H

// This is the main header to include to make an embedding of liblucet. It is
// simply an umbrella header. See documentation in the individual headers for
// more info.

// Methods on lucet_pool. A lucet_pool allocates memory for efficient reuse of
// instances. Use these methods to create and destroy a pool.
#include "lucet_pool.h"

// Methods on lucet_module. A lucet_module is the compiled code from a single wasm
// module. It needs to be instantiated in order to be run. Use these methods to
// load and unload modules stored in shared object files (.so) in the
// filesystem.
#include "lucet_module.h"

// Methods on lucet_instance. An instance is used to run guest code. Use these
// methods to create one for a given module, run it with some given arguments,
// manipulate its memory before or after running it, and examine its state when
// it exits.
#include "lucet_instance.h"

// Methods on lucet_vmctx. Host calls (functions in the embedding, also called the
// host, that are called by the guest) always get a lucet_vmctx* as their first
// argument. These methods can be used from inside a host call to manipulate the
// guest memory, or it to exit.
#include "lucet_vmctx.h"

// Methods on lucet_state, which describes the state of an lucet_instance. Use these
// methods to examine the reason an instance terminated.
#include "lucet_state.h"

// Methods for collecting statistics on liblucet. Register a callback to recieve a
// message whenever an event happens in liblucet.
#include "lucet_stats.h"

// Useful constants. In a separate header so that users can pare down imports.
#include "lucet_constants.h"

// Macros to convert between typed values (lucet_val) and native types
#include "lucet_val.h"

#endif // VMAPI_H
