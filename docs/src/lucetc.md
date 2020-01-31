# Lucetc

`lucetc` is the Lucet Compiler.

The Rust crate `lucetc` provides an executable `lucetc`.

It compiles WebAssembly modules (`.wasm` or `.wat` files) into native code (`.o` or `.so` files).

## Example

```sh
lucetc example.wasm --output example.so --bindings lucet-wasi/bindings.json --reserved-size 64MiB --opt-level best
```

This command compiles `example.wasm`, a WebAssembly module, into a shared library `example.so`. At
run time, the heap can grow up to 64 MiB.

Lucetc can produce ELF (on Linux) and Mach-O (on macOS) objects and libraries. For debugging
purposes or code analysis, it can also dump Cranelift code.

## Usage

```text
    lucetc [FLAGS] [OPTIONS] [--] [input]

FLAGS:
        --count-instructions    Instrument the produced binary to count the number of wasm operations the translated
                                program executes
    -h, --help                  Prints help information
        --signature-keygen      Create a new key pair
        --signature-create      Sign the object file
    -V, --version               Prints version information
        --signature-verify      Verify the signature of the source file

OPTIONS:
        --bindings <bindings>...                   path to bindings json file
        --builtins <builtins>                      builtins file
        --emit <emit>
            type of code to generate (default: so) [possible values: obj, so, clif]

        --guard-size <guard_size>                  size of linear memory guard. must be multiple of 4k. default: 4 MiB
        --max-reserved-size <max_reserved_size>
            maximum size of usable linear memory region. must be multiple of 4k. default: 4 GiB

        --min-reserved-size <min_reserved_size>
            minimum size of usable linear memory region. must be multiple of 4k. default: 4 MiB

        --opt-level <opt_level>
            optimization level (default: 'speed_and_size'). 0 is alias to 'none', 1 to 'speed', 2 to 'speed_and_size'
            [possible values: 0, 1, 2, none, speed, speed_and_size]
    -o, --output <output>                          output destination, defaults to a.out if unspecified
        --signature-pk <pk_path>                   Path to the public key to verify the source code signature
        --precious <precious>                      directory to keep intermediate build artifacts in
        --reserved-size <reserved_size>
            exact size of usable linear memory region, overriding --{min,max}-reserved-size. must be multiple of 4k

        --signature-sk <sk_path>
            Path to the secret key to sign the object file. The file can be prefixed with "raw:" in order to store a
            raw, unencrypted secret key

ARGS:
    <input>    input file

```

## External symbols

By default, compiled files cannot call any external function. Not even WASI's. Allowed external
functions have to be explicitly listed in bindings JSON file, that have the following format:

```json
{
    "wasi_unstable": {
        "symbol_name_1": "native_symbol_name_1",
        "symbol_name_n": "native_symbol_name_n",
    }
}
```

The example above allows the WebAssembly module to refer to an external symbol `symbol_name_1`, that
maps to the native symbol `native_symbol_name_1`.

The `--bindings` command-line switch can be used more than once in order to split the definitions
into multiple files.

When using WASI, the `bindings.json` file shipped with `lucet-wasi` can be used in order to import
all the symbols available in the `lucet-wasi` runtime.

## Memory limits

* `--max-reserved-size <size>` makes the compiler assume that the heap will never grow more than
  `<size>` bytes. The compiler will generate code optimized for that size, inserting bound checks
  with static values whenever necessary. As a side effect, the module will trap if the limit is ever
  reached, even if the runtime could allow the heap to grow even further.

* `--min-reserved-size <size>` sets the maximum heap size the runtime should use.

* `--reserved-size <size>` is a shortcut to set both values simultaneously, and is the recommended
  way to configure how much memory the module can use. The default is only 4 MiB, so this is
  something you may want to increase.

* `--guard-size <size>` controls how much virtual memory with no read nor write access is reserved
  after an instance's heap. The compiler can avoid some bound checking when it is safe to do so
  according to this value.

## Optimization levels

* `--opt-level 0` makes the compilation as fast as possible, but the resulting code itself may not
  be optimal.

* `--opt-level 1` generates fast code, but does not run passes intended to reduce code size.

* `--opt-level 2` generates the fastest and smallest, but is compilation is about twice as slow as
  `0`.

## Builtins

`lucetc` can replace internal functions with calls to external, optimized implementations; see
[`lucet-builtins`](./lucet-builtins.md).
