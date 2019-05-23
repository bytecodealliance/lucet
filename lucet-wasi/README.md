# lucet-wasi

Experimental WASI embedding for the Lucet runtime.

## Examples

Example WASI programs are in the [`examples`](examples) directory.

## Supported syscalls

We support the entire [WASI API](https://github.com/CraneStation/wasmtime/blob/master/docs/WASI-api.md),
with the exception of socket-related syscalls. These will be added when
network access is standardized.

## Thread safety

Lucet guests are currently single-threaded only. The WASI embedding assumes
this, and so the syscall implementations are not thread-safe. This is not a
fundamental limitation, should Lucet support multi-threaded guests in the
future.

## TODOs

### Introduce optional abstraction between system clocks and WASI clocks

The current implementations of `__wasi_clock_*` delegate to the host system's
`clock_getres` and `clock_gettime`. For untrusted code, it would be useful to
limit the precision of these clocks to reduce the potential impact of side
channels. Furthermore, the `CLOCK_*_CPUTIME_ID` clocks currently give timings
for the host process, but a measure of the guest instance runtime would be
more accurate.

### Rewrite the code that implements capability checking

Much of this code is a direct port of the [`wasmtime` C implementation](https://github.com/CraneStation/wasmtime/tree/master/wasmtime-wasi/sandboxed-system-primitives),
and as such contains a fair amount of unsafety and low-level operations on
bytestrings and bitfields. Since this code is critical to the integrity of the
sandboxing model, we intend to rewrite this code in higher-level Rust that is
easier to test and verify.

## Third-Party Code

`src/wasm32.rs` is copied from
[wasmtime](https://github.com/CraneStation/wasmtime/blob/master/wasmtime-wasi/src/wasm32.rs),
along with the associated `LICENSE.wasmtime` file.

Parts of our syscall implementations are derived from the C implementations in
`cloudabi-utils`. See `LICENSE.cloudabi-utils` for license information.
