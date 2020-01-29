# Versioning and releasing to crates.io

This document describes how to appropriately decide the version of a new release, and the steps to
actually get it published on crates.io.

## Versioning

We release new versions of the Lucet crates all at once, keeping the versions in sync across
crates. As a result, our adherence to [semver](https://semver.org/) is project-wide, rather than
per-crate.

The versioning reflects the semantics of the *public* interface to Lucet. That is, any breaking
change to the following crates requires a semver major version bump:

- `lucetc`
- `lucet-objdump`
- `lucet-runtime`
- `lucet-validate`
- `lucet-wasi`
- `lucet-wasi-sdk`

For the other Lucet crates that are primarily meant for internal consumption, a breaking change does
*not* inherently require a semver major version bump unless either:

1. The changed interfaces are reexported as part of the public interface via the above crates, or
2. The binary format of a compiled Lucet module is changed.

For example, a change to the type of [`Instance::run()`][public-method] would require a major
version bump, but a change to the type of [`RegionInternal::expand_heap()`][internal-method] would
not.

[public-method]: https://docs.rs/lucet-runtime-internals/latest/lucet_runtime_internals/instance/struct.Instance.html#method.run
[internal-method]: https://docs.rs/lucet-runtime-internals/latest/lucet_runtime_internals/instance/trait.InstanceInternal.html#tymethod.alloc

Likewise, a change to a field on [`ModuleData`][module-data] would require a major version bump, as
it would change the serialized representation in a compiled Lucet module.

[module-data]: https://docs.rs/lucet-module/latest/lucet_module/struct.ModuleData.html

## Releasing

Releasing a workspace full of interdependent crates can be challenging. Crates must be published in
the correct order, and any cyclic dependencies that might be introduced via `[dev-dependencies]`
must be broken. While there is [interest in making this smoother][publish-dev-deps], for now we have
to muddle through more manually.

[publish-dev-deps]: https://github.com/rust-lang/cargo/issues/4242

1. Authenticate with `cargo login` using a Github account with the appropriate access to the Lucet
   repository. You should only have to do this once per development environment.

1. Ensure that you have the commit checked out that you would like to release.

1. Ensure that the version in all of the Lucet `Cargo.toml` files matches the version you expect to
   release. Between releases, the versions will end in `-dev`; if this is still the case, you'll
   need to replace this version with the appropriate version according to the guidelines above,
   likely through a PR.

1. Edit `lucet-validate/Cargo.toml` and make the following change:

   ```diff
    [dev-dependencies]
   -lucet-wasi-sdk = { path = "../lucet-wasi-sdk", version = "=0.5.2-dev" }
   +#lucet-wasi-sdk = { path = "../lucet-wasi-sdk", version = "=0.5.2-dev" }
    tempfile = "3.0"
   ```
   
   This breaks the only cycle that exists among the crates as of `0.5.1`; if other cycles develop,
   you'll need to similarly break them by temporarily removing the dev dependency.

1. Begin publishing the crates in a topological order by `cd`ing to the each crate and running
   `cargo publish --allow-dirty` (the tree should only be dirty due to the cycles broken
   above). While we would like to run `cargo publish --dry-run` beforehand to ensure all
   of the crates will be successfully published, this will fail for any crates that depend on other
   Lucet crates, as the new versions will not yet be available to download.

   Do not worry too much about calculating the order ahead of time; if you get it wrong, `cargo
   publish` will tell you which crates need to be published before the one you tried. An order which
   worked for the `0.5.1` release was:
   
   1. `lucet-module`
   1. `lucet-validate`
   1. `lucetc`
   1. `lucet-wasi-sdk`
   1. `lucet-objdump`
   1. `lucet-runtime-macros`
   1. `lucet-runtime-internals`
   1. `lucet-runtime-tests`
   1. `lucet-runtime`
   1. `lucet-wasi`

   It is unlikely but not impossible that a publish will fail in the middle of this process, leaving
   some of the crates published but not others. What to do next will depend on the situation; please
   consult with the Lucet team.

1. Undo any changes in your local tree to break cycles.

1. Tag the release; `--sign` is optional but recommended if you have code signing configured:

   ```shell
   $ git tag --annotate --sign -m '0.5.1 crates.io release' 0.5.1
   $ git push --tags
   ```
