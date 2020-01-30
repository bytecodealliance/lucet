### Unreleased

### 0.5.1 (2020-01-24)

- Fixed a memory corruption bug that could arise in certain runtime
  configurations. ([PR](https://github.com/bytecodealliance/lucet/pull/401)) ([RustSec
  advisory](https://rustsec.org/advisories/RUSTSEC-2020-0004.html))

### 0.5.0 (2020-01-24)

- Lucet officially became a project of the [Bytecode Alliance](https://bytecodealliance.org/) 🎉.

- Integrated `wasi-common` as the underlying implementation for WASI in `lucet-wasi`.

- Updated to Cranelift to version 0.51.0.

- Fixed a soundness bug by changing the types of the `Vmctx::yield*()` methods to require exclusive
  `&mut self` access to the `Vmctx`. This prevents resources like embedder contexts or heap views
  from living across yield points, which is important for safety since the host can modify the data
  underlying those resources while the instance is suspended.

- Added the `#[lucet_hostcall]` attribute to replace `lucet_hostcalls!`, which is now deprecated.

- Added the ability to specify an alignment for the base of a `MmapRegion`-backed instance's
  heap. Thanks, @shravanrn!

- Added a `--target` option to `lucetc` to allow cross-compilation to other architectures than the
  host's. Thanks, @froydnj!

- Changed the Cargo dependencies between Lucet crates to be exact (e.g., `"=0.5.0"` rather than
  `"0.5.0"`) rather than allowing semver differences.

- Fixed the `KillSwitch` type not being exported from the public API, despite being usable via
  `Instance::kill_switch()`.

- Improved the formatting of error messages.

- Ensured the `lucet-wasi` executable properly links in the exported symbols from `lucet-runtime`.

### 0.4.3 (2020-01-24)

- Backported the fix for a memory corruption bug that could arise in certain runtime
  configurations. ([PR](https://github.com/bytecodealliance/lucet/pull/401)) ([RustSec
  advisory](https://rustsec.org/advisories/RUSTSEC-2020-0004.html))
