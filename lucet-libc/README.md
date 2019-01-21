# fst-musl-wasm

This library is a subset of musl libc, built as WebAssembly, for use with liblucet-runtime-c.

## Building

A top level Makefile is used as a driver for an underlying ninja build. The
script `src/build.py` will create a ninja build file, and then execute ninja to
build all of the artifacts, in the directory provided by the `--out-dir`
argument.  The `--version` argument sets the version of the debian package
built by this script. You don't need to set a valid version when you are
building with cargo because the deb is not exposed.

Not all of the functions typically provided by musl are included in this build.
The file at `src/manifest.py` defines (and has comments on) which sections of
the library are provided or not.

## Debian Packaging

The debian package is provided for deploying to production, i.e. for using this
library in another devly project or on a builder.

You should update the debian package revision by changing the
`FST_MUSL_WASM_VERSION` definition in `package.sh`. Invoke `package.sh` with an
argument giving the directory to write the deb file to.

The deb can be used to install the library to `/opt/fst-musl-wasm/`. The `lib/` subdir
will contain `libc.a` and the `include/` subdir will contain the libc headers.

## Dependencies

Depends on a clang >= 6 that has the wasm32 backend enabled. Will default to using `clang`, `wasm-ld`, and `llvm-ar` available in the path. Override these with the `LUCET_CLANG`, `LUCET_WASM_LD`, and `LUCET_LLVM_AR` environment variables to point at a non-PATH installation of clang.

## Sources

### musl

We have an internal fork of the public musl source repository
(https://git.musl-libc.org/cgit/musl) at fastly/musl. We use a git submodule
to make this available in `src/musl`. The submodule should check out a release tag
from upstream. Initially the project used release 1.1.19, the latest at that time.

### compiler-rt

While not typically part of libc, I didn't find any pressing need to go through
the trouble to package these separately,, so the primitives needed by clang
that typically come from compiler-rt are also provided in `libc.a`. The sources
are provided as a git submodule at `src/compiler-rt`.


### wasm32 Architecture headers

The musl headers specific to the wasm32 architecture are provided at `src/arch/wasm32`.
These headers are derived from the ones found in https://github.com/jfbastien/musl.


### wasm32\_rt

Some parts of musl are not appropriate for the wasm32 or liblucet-runtime-c environment.
Some are just left out, but some got replacements in this directory. We
implement stdio with the liblucet-runtime-c debug hook. Instead of using the allocator
provided by musl (which depends on various mmap/mprotect syscalls that we cant
provide), we implement a much simpler allocator based on the lend allocator by
whitequark for bookkeeping with a simple free list. We use the WebAssembly
expand memory primitive to expand lend's memory arena. This simple allocator is
probably sufficient for the small footprint and short lifetime of the use cases
we have in mind for liblucet-runtime-c but if that were to change this choice should be
reconsidered.

