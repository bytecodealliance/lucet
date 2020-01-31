# `lucet-runtime` &nbsp; [![docs-badge]][docs-rs]

[docs-badge]: https://docs.rs/lucet-runtime/badge.svg
[docs-rs]: https://docs.rs/lucet-runtime

`lucet-runtime` is the runtime for WebAssembly modules compiled through `lucetc`.

It is a Rust crate that provides the functionality to load modules from shared object files,
instantiate them, and call exported WebAssembly functions. `lucet-runtime` manages the resources
used by each WebAssembly instance (linear memory & globals), and the exception mechanisms that
detect and recover from illegal operations.

The public API of the library is defined `lucet-runtime`, but the bulk of the implementation is in
the child crate `lucet-runtime-internals`. Proc macros are defined in `lucet-runtime-macros`, and
test suites are defined in the child crate `lucet-runtime-tests`. Many of these tests invoke
`lucetc` and the `wasi-sdk` tools.

`lucet-runtime` is usable as a Rust crate or as a C library. The C language interface is found at
`lucet-runtime/include/lucet.h`.
