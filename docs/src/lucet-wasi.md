# `lucet-wasi` &nbsp; [![docs-badge]][docs-rs]

[docs-badge]: https://docs.rs/lucet-wasi/badge.svg
[docs-rs]: https://docs.rs/lucet-wasi

`lucet-wasi` is a crate providing runtime support for the [WebAssembly System Interface
(WASI)](https://wasi.dev).  It can be used as a library to support WASI in another application, or
as an executable, `lucet-wasi`, to execute WASI programs compiled through `lucetc`.

Example WASI programs are in the [`examples`](examples) directory.

## Example

```sh
lucet-wasi example.so --dir .:. --max-heap-size 2GiB -- example_arg
```

## Usage

```text
    lucet-wasi [OPTIONS] <lucet_module> [--] [guest_args]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --entrypoint <entrypoint>                         Entrypoint to run within the WASI module [default: _start]
        --heap-address-space <heap_address_space_size>
            Maximum heap address space size (must be a multiple of 4 KiB, and >= `max-heap-size`) [default: 8 GiB]

        --max-heap-size <heap_memory_size>
            Maximum heap size (must be a multiple of 4 KiB) [default: 4 GiB]

        --dir <preopen_dirs>...                           A directory to provide to the WASI guest
        --stack-size <stack_size>
            Maximum stack size (must be a multiple of 4 KiB) [default: 8 MiB]


ARGS:
    <lucet_module>     Path to the `lucetc`-compiled WASI module
    <guest_args>...    Arguments to the WASI `main` function
```

## Preopened files and directories

By default, WASI doesn't allow any access to the filesystem. Files and directories must be
explicitly allowed by the host.

Instead of directly accessing the filesystem using paths, an instance will use inherited descriptors
from pre-opened files.

This means that even the current directory cannot be accessed by a WebAssembly instance, unless this
has been allowed with:

```text
--dir .:.
```

This maps the current directory `.` as seen by the WebAssembly module to `.` as seen on the host.

Multiple `--dir <wasm path>:<host path>` arguments can be used in order to allow the instance to
access more paths.

Along with a preopened file/directory, WASI stores a set of capabilities. Lucet currently sets all
the capabilities. In particular, once a directory has been preopened, its content as well as files
from any of its subdirectories can be accessed as well.

## Maximum heap size

`--heap-address-space` controls the maximum allowed heap size.

Usually, this should match the `--reserved-size` value given to `lucetc`.

## Supported syscalls

We support the entire [WASI
API](https://github.com/bytecodealliance/wasmtime/blob/main/docs/WASI-api.md), with the exception of
socket-related syscalls. These will be added when network access is standardized.

## Thread safety

Lucet guests are currently single-threaded only. The WASI embedding assumes this, and so the syscall
implementations are not thread-safe. This is not a fundamental limitation, should Lucet support
multi-threaded guests in the future.
