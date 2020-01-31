# Example usage of the `lucet-runtime` crate.

The following Rust code loads a WebAssembly module compiled using `lucetc-wasi` and calls its
`main()` function.

## `.cargo/config`:

These flags must be set in order to have symbols from the runtime properly exported.

```toml
{{#include ../lucet-runtime-example/.cargo/config}}
```

## `Cargo.toml`:

```toml
{{#include ../lucet-runtime-example/Cargo.toml}}
```

## `src/main.rs`:

```rust
{{#include ../lucet-runtime-example/src/main.rs}}
```
