# Lucet has reached End-of-life

Lucet has reached end-of-life, and maintence has ceased. All Lucet users
should transition to [Wasmtime][wasmtime].

In mid-2020, the Lucet team switched focus to working on the
[Wasmtime][wasmtime] engine. We have added all of the features to Wasmtime
which previously only Lucet had, such as ahead-of-time (AOT) compilation and a
pooling userfaultfd-based memory allocator.

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

