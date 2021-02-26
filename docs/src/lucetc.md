# `lucetc` &nbsp; [![docs-badge]][docs-rs]

[docs-badge]: https://docs.rs/lucetc/badge.svg
[docs-rs]: https://docs.rs/lucetc

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
        --count-instructions
            Instrument the produced binary to count the number of wasm operations the translated program executes

    -h, --help
            Prints help information

        --signature-keygen
            Create a new key pair

        --no-translate-wat
            Disable translating wat input files to wasm

        --signature-create
            Sign the object file

    -V, --version
            Prints version information

        --signature-verify
            Verify the signature of the source file

        --wasi_exe
            validate as a wasi executable

        --wiggle-bindings
            use wiggle to calculate bindings


OPTIONS:
        --bindings <bindings>...
            path to bindings json file

        --emit <emit>
            type of code to generate (default: so) [possible values: obj, so, clif]

        --error-style <error_style>
            Style of error reporting (default: human) [possible values: human, json]

        --guard-size <guard_size>
            size of linear memory guard. must be multiple of 4k. default: 4 MiB

        --min-os-version <min_os_version>
            Minimum macOS version to support

        --opt-level <opt_level>
            optimization level (default: 'speed_and_size'). 0 is alias to 'none', 1 to 'speed', 2 to 'speed_and_size'
            [possible values: 0, 1, 2, none, speed, speed_and_size]
    -o, --output <output>
            output destination, defaults to a.out if unspecified

        --signature-pk <pk_path>
            Path to the public key to verify the source code signature

        --precious <precious>
            directory to keep intermediate build artifacts in

        --reserved-size <reserved_size>
            exact size of usable linear memory region, overriding --{min,max}-reserved-size. must be multiple of 4k

        --sdk-version <sdk_version>
            MacOS SDK version to support

        --signature-sk <sk_path>
            Path to the secret key to sign the object file. The file can be prefixed with "raw:" in order to store a
            raw, unencrypted secret key
        --target <target>
            target to compile for, defaults to x86_64-apple-darwin if unspecified

        --target-cpu <target-cpu>
            Generate code for a particular type of CPU.

            If neither `--target-cpu` nor `--target-feature` is provided, `lucetc`
            will automatically detect and use the features available on the host CPU.
            This is equivalent to choosing `--target-cpu=native`.

             [possible values: native, baseline, nehalem, sandybridge, haswell, broadwell, skylake, cannonlake, icelake,
            znver1]
        --target-feature <target-feature>...
            Enable (+) or disable (-) specific CPU features.

            If neither `--target-cpu` nor `--target-feature` is provided, `lucetc`
            will automatically detect and use the features available on the host CPU.

            This option is additive with, but takes precedence over `--target-cpu`.
            For example, `--target-cpu=haswell --target-feature=-avx` will disable
            AVX, but leave all other default Haswell features enabled.

            Multiple `--target-feature` groups may be specified, with precedence
            increasing from left to right. For example, these arguments will enable
            SSE3 but not AVX:

                --target-feature=+sse3,+avx --target-feature=-avx

             [possible values: +sse3, -sse3, +ssse3, -ssse3, +sse41, -sse41, +sse42, -sse42, +popcnt, -popcnt, +avx,
            -avx, +bmi1, -bmi1, +bmi2, -bmi2, +lzcnt, -lzcnt]
        --witx <witx_specs>...
            path to witx spec to validate against


ARGS:
    <input>
            input file
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

* `--reserved-size <size>` indicates how much virtual memory (not including guard pages) will be
  reserved by the runtime for the module's linear memory. The default is 4GiB. A smaller value
  would prevent bounds checking elision, significantly reducing performance.

* `--guard-size <size>` controls how much virtual memory with no read nor write access is reserved
  after an instance's heap. The compiler can avoid some bound checking when it is safe to do so
  according to this value.

## Optimization levels

* `--opt-level 0` makes the compilation as fast as possible, but the resulting code itself may not
  be optimal.

* `--opt-level 1` generates fast code, but does not run passes intended to reduce code size.

* `--opt-level 2` generates the fastest and smallest, but is compilation is about twice as slow as
  `0`.
