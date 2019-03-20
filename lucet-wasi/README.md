# lucet-wasi

Experimental WASI embedding for the Lucet runtime.

## TODOs

In addition to the [WASI
syscalls](https://github.com/CraneStation/wasmtime/blob/wasi/docs/WASI-api.md) we haven't yet
defined:

### Introduce optional abstraction between system clocks and WASI clocks

The current implementations of `__wasi_clock_*` delegate to the host system's `clock_getres` and
`clock_gettime`. For untrusted code, it would be useful to limit the precision of these clocks to
reduce the potential impact of side channels. Furthermore, the `CLOCK_*_CPUTIME_ID` clocks currently
give timings for the host process, but a measure of the guest instance runtime would be more
accurate.

## Third-Party Code

`src/wasm32.rs` is copied from
[wasmtime-wasi](https://github.com/CraneStation/wasmtime/blob/wasi/wasmtime-wasi/src/wasm32.rs),
along with the associated `LICENSE.wasmtime-wasi` file.
