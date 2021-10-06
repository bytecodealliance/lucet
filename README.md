## Lucet is in Maintence Mode

Since mid 2020, the Lucet team has been focusing our efforts on the
[Wasmtime][wasmtime] project. Wasmtime now has support for all of the features
that previously only Lucet had, such as ahead-of-time (AOT) compilation
and a pooling userfaultfd-based memory allocator.

We encourage all new projects to use [Wasmtime][wasmtime], and existing Lucet
users to transition to Wasmtime.

[wasmtime]: https://github.com/bytecodealliance/wasmtime

# Lucet &nbsp; [![Build Status]][gh-actions]

[Build Status]: https://github.com/bytecodealliance/lucet/workflows/CI/badge.svg
[gh-actions]: https://github.com/bytecodealliance/lucet/actions?query=workflow%3ACI

**A [Bytecode Alliance][BA] project**

[BA]: https://bytecodealliance.org/

**Lucet is a native WebAssembly compiler and runtime. It is designed
to safely execute untrusted WebAssembly programs inside your application.**

Check out our [announcement post on the Fastly blog][announce-blog].

[announce-blog]: https://www.fastly.com/blog/announcing-lucet-fastly-native-webassembly-compiler-runtime

Lucet uses, and is developed in collaboration with, the Bytecode Alliance's
[Cranelift](http://github.com/bytecodealliance/cranelift) code generator. It powers Fastly's
[Compute@Edge](https://www.fastly.com/products/edge-compute/serverless) platform.

[![asciicast](https://asciinema.org/a/249302.svg)](https://asciinema.org/a/249302)

Lucet's documentation is available at <https://bytecodealliance.github.io/lucet>
([sources](https://github.com/bytecodealliance/lucet/tree/main/docs)).

