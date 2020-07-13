### Unreleased

- Added `install_lucet_signal_handler()` and `remove_lucet_signal_handler()`, along with `Instance::ensure_signal_handler_installed()` and `Instance::ensure_sigstack_installed()` options to control the automatic installation and removal of signal handlers and alternate signal stacks. The default behaviors have not changed.

- Added `Instance::run_start()` to the public API, which runs the [Wasm start function][start-function] if it is present in that instance's Wasm module. It does nothing if there is no start function.

  Creating or resetting an instance no longer implicitly runs the start function. Embedders must ensure that `run_start()` is called before calling any other exported functions. `Instance::run()` will now return `Err(Error::InstanceNeedsStart)` if the start function is present but hasn't been run since the instance was created or reset.

- Encoded the [Wasm start function][start-function] in Lucet module metadata, rather than as a specially-named symbol in the shared object. This reduces contention from `dlsym` operations when multiple threads run Lucet instances concurrently.

- Upgraded the `libloading` dependency, allowing for more specific error messages from dynamic loading operations.

- Corrected a race condition where a `KillSwitch` fired while lucet-runtime is handling a guest fault could result in a SIGALRM or panic in the Lucet embedder.

- Converted the `&mut Vmctx` argument to hostcalls into `&Vmctx`. Additionally, all `Vmctx` methods now take `&self`, where some methods such as `yield` previously took `&mut self`. These methods still require that no other outstanding borrows (such as the heap view) are held across them, but that property is checked dynamically rather than at compile time.

- Added the field `hostcall_reservation` to `Limits` to specify an amount of stack space Lucet will ensure is available when making a hostcall. `hostcall_reservation` defaults to 32KiB. If there is less than the configured amount of stack space when making a hostcall, the instance will fault in the same way as any other guest-code stack overflow.

[start-function]: https://webassembly.github.io/spec/core/syntax/modules.html#syntax-start

### 0.6.1 (2020-02-18)

- Added metadata to compiled modules that record whether instruction counting instrumentation is present.

- Made `lucetc` more flexible in its interpretation of the `LD` environment variable. It now accepts a space-separated set of tokens; the first token specifies the program to invoke, and the remaining tokens specifying arguments to be passed to that program. Thanks, @froydnj!

- Added public `LucetcOpt` methods to configure the `canonicalize_nans` setting. Thanks, @roman-kashitsyn!

- Fixed `lucet-runtime`'s use of CPUID to not look for extended features unless required by the module being loaded, avoiding a failure on older CPUs where that CPUID leaf is not present. Thanks, @shravanrn!

### 0.6.0 (2020-02-05)

- Added `free_slots()`, `used_slots()`, and `capacity()` methods to the `Region` trait.

- Added a check to ensure the `Limits` signal stack size is at least `MINSIGSTKSZ`, and increased the default signal stack size on macOS debug builds to fit this constraint.

- Added an option to canonicalize NaNs to the `lucetc` API. Thanks, @DavidM-D!

- Restored some of the verbosity of pretty-printed errors in `lucetc` and `lucet-validate`, with more on the way.

- Fixed OS detection for LDFLAGS on macOS. Thanks, @roman-kashitsyn!

### 0.5.1 (2020-01-24)

- Fixed a memory corruption bug that could arise in certain runtime configurations. ([PR](https://github.com/bytecodealliance/lucet/pull/401)) ([RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2020-0004.html))

### 0.5.0 (2020-01-24)

- Lucet officially became a project of the [Bytecode Alliance](https://bytecodealliance.org/) 🎉.

- Integrated `wasi-common` as the underlying implementation for WASI in `lucet-wasi`.

- Updated to Cranelift to version 0.51.0.

- Fixed a soundness bug by changing the types of the `Vmctx::yield*()` methods to require exclusive `&mut self` access to the `Vmctx`. This prevents resources like embedder contexts or heap views
  from living across yield points, which is important for safety since the host can modify the data underlying those resources while the instance is suspended.

- Added the `#[lucet_hostcall]` attribute to replace `lucet_hostcalls!`, which is now deprecated.

- Added the ability to specify an alignment for the base of a `MmapRegion`-backed instance's heap. Thanks, @shravanrn!

- Added a `--target` option to `lucetc` to allow cross-compilation to other architectures than the host's. Thanks, @froydnj!

- Changed the Cargo dependencies between Lucet crates to be exact (e.g., `"=0.5.0"` rather than `"0.5.0"`) rather than allowing semver differences.

- Fixed the `KillSwitch` type not being exported from the public API, despite being usable via `Instance::kill_switch()`.

- Improved the formatting of error messages.

- Ensured the `lucet-wasi` executable properly links in the exported symbols from `lucet-runtime`.

### 0.4.3 (2020-01-24)

- Backported the fix for a memory corruption bug that could arise in certain runtime configurations. ([PR](https://github.com/bytecodealliance/lucet/pull/401)) ([RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2020-0004.html))
