# Your first Lucet application

Ensure the Lucet command-line tools are [installed on your machine](./Compiling.md)

Create a new work directory in the `lucet` directory:

```sh
mkdir -p src/hello

cd src/hello
```

Save the following C source code as `hello.c`:

```c
#include <stdio.h>

int main(void)
{
    puts("Hello world");
    return 0;
}
```

Time to compile to WebAssembly! The development environment includes a version of the Clang
toolchain that is built to generate WebAssembly by default. The related commands are accessible from
your current shell, and are prefixed by `wasm32-wasi-`.

For example, to create a WebAssembly module `hello.wasm` from `hello.c`:

```sh
wasm32-wasi-clang -Ofast -o hello.wasm hello.c
```

The next step is to use Lucet to build native `x86_64` code from that WebAssembly file:

```sh
lucetc-wasi -o hello.so hello.wasm
```

`lucetc` is the WebAssembly to native code compiler. The `lucetc-wasi` command runs the same
compiler, but automatically configures it to target WASI.

`hello.so` is created and ready to be run:

```sh
lucet-wasi hello.so
```
