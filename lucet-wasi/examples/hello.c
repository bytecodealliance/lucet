// To compile this code, use a clang toolchain built by
// [wasmception-wasi](https://github.com/CraneStation/wasmception-wasi/). Because that's a rather
// involved process, we include the `clang -Os` output for the example in WebAssembly text format
// (`hello.wat`).

#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv)
{
    char *greeting = getenv("GREETING");
    if (greeting == NULL) {
        greeting = "hello";
    }

    if (argc < 2) {
        printf("%s, wasi!\n", greeting);
    } else {
        printf("%s, %s!\n", greeting, argv[1]);
    }
}
