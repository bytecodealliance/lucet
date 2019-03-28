# lucet-wasi

Experimental WASI embedding for the Lucet runtime.

Much of this code is a direct port of the `cloudabi-utils`-based syscall emulation layer via
[`wasmtime`](https://github.com/CraneStation/wasmtime/tree/master/wasmtime-wasi/sandboxed-system-primitives). It
is currently suitable for demonstration purposes, but needs to be rewritten in a more Rust-native
style to reduce the code complexity and the number of potential panics.

If you have questions or suggestions, the authors of `lucet-wasi` and others in the WASI community
can be found in [`#wasi` on Mozilla IRC](https://wiki.mozilla.org/IRC).

## Examples

Example WASI programs are in the [`examples`](examples) directory.

## Supported syscalls

We support a subset of the [WASI
API](https://github.com/CraneStation/wasmtime/blob/master/docs/WASI-api.md), though we are adding
new syscalls on a regular basis. We currently implement:

- `__wasi_args_get`
- `__wasi_args_sizes_get`
- `__wasi_clock_res_get`
- `__wasi_clock_time_get`
- `__wasi_environ_get`
- `__wasi_environ_sizes_get`
- `__wasi_fd_close`
- `__wasi_fd_fdstat_get`
- `__wasi_fd_fdstat_set_flags`
- `__wasi_fd_prestat_dir_name`
- `__wasi_fd_prestat_get`
- `__wasi_fd_read`
- `__wasi_fd_seek`
- `__wasi_fd_write`
- `__wasi_path_open`
- `__wasi_proc_exit`
- `__wasi_random_get`

This is enough to run basic C and Rust programs, including those that use command-line arguments,
environment variables, stdio, and basic file operations.

## Thread safety

Lucet guests are currently single-threaded only. The WASI embedding assumes this, and so the syscall
implementations are not thread-safe. This is not a fundamental limitation, should Lucet support
multi-threaded guests in the future.

## TODOs

### Complete the WASI API syscalls

We are missing support for advanced filesystem operations, sockets, and polling, among others.

### Introduce optional abstraction between system clocks and WASI clocks

The current implementations of `__wasi_clock_*` delegate to the host system's `clock_getres` and
`clock_gettime`. For untrusted code, it would be useful to limit the precision of these clocks to
reduce the potential impact of side channels. Furthermore, the `CLOCK_*_CPUTIME_ID` clocks currently
give timings for the host process, but a measure of the guest instance runtime would be more
accurate.

### Rewrite the code that implements capability checking

Much of this code is a direct port of the [`wasmtime` C
implementation](https://github.com/CraneStation/wasmtime/tree/master/wasmtime-wasi/sandboxed-system-primitives),
and as such contains a fair amount of unsafety and low-level operations on bytestrings and
bitfields. Since this code is critical to the integrity of the sandboxing model, we intend to
rewrite this code in higher-level Rust that is easier to test and verify.

## Third-Party Code

`src/wasm32.rs` is copied from
[wasmtime](https://github.com/CraneStation/wasmtime/blob/master/wasmtime-wasi/src/wasm32.rs), along
with the associated `LICENSE.wasmtime` file.

Significant parts of our syscall implementations are derived from the C implementations in
`cloudabi-utils`. See `LICENSE.cloudabi-utils` for license information.
